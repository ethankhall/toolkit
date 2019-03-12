use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{stdout, Stdout, Write};
use std::{thread, time};

use chrono::prelude::*;
use clap::ArgMatches;
use colored::*;
use prettytable::{format, Table};
use termion::screen::*;

use crate::commands::nsq::api::*;
use crate::commands::CliError;

pub fn do_status_command(matches: &ArgMatches) -> Result<(), CliError> {
    let topics = matches
        .values_of("TOPIC")
        .map(|x| x.collect())
        .unwrap_or_else(|| vec![]);

    let nsq_lookup_host = matches.value_of("nsq_lookup_host").unwrap();
    let nsq_lookup_port = matches.value_of("nsq_lookup_port").unwrap();

    let count = matches.value_of("count").map(|x| x.parse::<u32>().unwrap());

    let nsq_lookup = format!("{}:{}", nsq_lookup_host, nsq_lookup_port);

    let mut topic_urls: Vec<(String, Vec<String>)> = Vec::new();
    for topic in topics {
        let base_urls = super::api::get_base_url_for_topic(&nsq_lookup, &topic);
        if base_urls.is_empty() {
            return Err(CliError::new("Unable to get NSQ Host", 2));
        }

        topic_urls.push((s!(topic), base_urls));
    }

    let mut screen = AlternateScreen::from(stdout());
    let mut counter = 0;
    let mut last_data = None;

    loop {
        write!(
            screen,
            "{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();
        let calculated = check_data(&topic_urls, last_data, &mut screen);

        let diff = chrono::Duration::seconds(1) - (Local::now() - calculated.poll_time());
        last_data = Some(calculated);
        let sleep_time = if diff < chrono::Duration::zero() {
            time::Duration::from_micros(0)
        } else {
            match diff.to_std() {
                Err(_) => time::Duration::from_secs(1),
                Ok(dur) => dur,
            }
        };

        thread::sleep(sleep_time);
        counter += 1;

        if let Some(limit) = count {
            if counter >= limit {
                break;
            }
        }
    }

    Ok(())
}

fn check_data(
    topic_url_list: &Vec<(String, Vec<String>)>,
    last_data: Option<NsqStats>,
    screen: &mut AlternateScreen<Stdout>,
) -> NsqStats {
    let stats = NsqStats::new();
    for (topic_name, base_urls) in topic_url_list.iter() {
        for base_url in base_urls {
            let data = super::api::get_topic_status(base_url, topic_name);
            stats.register(base_url, data)
        }
    }
    info!("Polled at {}", s!(stats.poll_time).bold());

    for (topic_name, host_table) in make_host_table(&stats, last_data) {
        writeln!(screen, "\n# {}", topic_name.bold()).unwrap();
        host_table.print(screen).unwrap();

        writeln!(screen, "").unwrap();
        make_channel_table(&stats, &topic_name)
            .print(screen)
            .unwrap();
    }

    stats
}

fn make_channel_table(stats: &NsqStats, topic: &str) -> Table {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row!["Channel Name", "Depth", "In Flight ✈️"]);

    for (channel_name, channel) in stats
        .topics
        .borrow()
        .get(topic)
        .unwrap()
        .channels
        .borrow()
        .iter()
    {
        table.add_row(row![
            channel_name.bold(),
            s!(channel.depth).bold(),
            s!(channel.in_flight_count).bold()
        ]);
    }

    table
}

fn make_host_table(current: &NsqStats, last: Option<NsqStats>) -> BTreeMap<String, Table> {
    let mut hosts_table: BTreeMap<String, Table> = BTreeMap::new();

    for (topic_name, details) in current.topics.borrow().iter() {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(row!["Host Name", "Depth", "Message Count"]);

        let hosts = details.hosts.borrow();
        for (host_name, host_details) in hosts.iter() {
            table.add_row(row![
                host_name.bold(),
                s!(host_details.depth).bold(),
                s!(host_details.message_count).bold()
            ]);
        }

        table.add_row(row![
            "Total".dimmed().yellow(),
            s!(details.total_depth).dimmed().yellow(),
            s!(details.total_message_count).dimmed().yellow()
        ]);

        if let Some(ref previous_stats) = last {
            if let Some(last_topic_stats) = previous_stats.topics.borrow().get(topic_name) {
                table.add_row(row![
                    "Change",
                    "",
                    details.total_message_count - last_topic_stats.total_message_count
                ]);
                let mps =
                    (last_topic_stats.total_message_count - details.total_message_count) as f64;
                let mps =
                    mps / (previous_stats.poll_time - current.poll_time).num_milliseconds() as f64;
                let mps = mps * 1000 as f64;
                table.add_row(row!["Rate", "", format!("{:.4} m/s", mps)]);
            }
        }

        hosts_table.insert(s!(topic_name), table);
    }

    hosts_table
}

struct ChannelMetrics {
    depth: i32,
    in_flight_count: i32,
}

impl ChannelMetrics {
    fn update(&mut self, depth: i32, in_flight_count: i32) {
        self.depth += depth;
        self.in_flight_count += in_flight_count;
    }
}

struct TopicMetrics {
    depth: i32,
    message_count: i32,
}

struct TopicStats {
    channels: RefCell<BTreeMap<String, ChannelMetrics>>,
    hosts: RefCell<BTreeMap<String, TopicMetrics>>,
    total_depth: i32,
    total_message_count: i32,
}

impl TopicStats {
    fn new() -> Self {
        return TopicStats {
            channels: RefCell::new(BTreeMap::new()),
            hosts: RefCell::new(BTreeMap::new()),
            total_depth: 0,
            total_message_count: 0,
        };
    }

    fn update(&mut self, host: String, details: TopicDetails) {
        let topic_metrics = TopicMetrics {
            depth: details.depth,
            message_count: details.message_count,
        };
        self.hosts.borrow_mut().insert(host, topic_metrics);

        self.total_depth += details.depth;
        self.total_message_count += details.message_count;

        for channel in details.channels {
            if !self.channels.borrow().contains_key(&channel.channel_name) {
                self.channels.borrow_mut().insert(
                    channel.channel_name.clone(),
                    ChannelMetrics {
                        depth: 0,
                        in_flight_count: 0,
                    },
                );
            }

            let mut channels = self.channels.borrow_mut();
            let channel_value = channels.get_mut(&channel.channel_name).unwrap();
            channel_value.update(channel.depth, channel.in_flight_count);
        }
    }
}

struct NsqStats {
    poll_time: DateTime<Local>,
    topics: RefCell<BTreeMap<String, TopicStats>>,
}

impl NsqStats {
    fn new() -> Self {
        return NsqStats {
            poll_time: Local::now(),
            topics: RefCell::new(BTreeMap::new()),
        };
    }

    fn poll_time(&self) -> DateTime<Local> {
        self.poll_time.clone()
    }

    fn register(&self, url: &str, details: Option<StatusTopicsDetails>) {
        let host = url::Url::parse(url).unwrap().host().unwrap().to_string();
        let details = match details {
            None => {
                warn!("Unable to access NSQ API for {}", host.clone());
                return;
            }
            Some(details) => details,
        };

        for topic in details.topics {
            if !self.topics.borrow().contains_key(&topic.topic_name) {
                self.topics
                    .borrow_mut()
                    .insert(topic.topic_name.clone(), TopicStats::new());
            }

            self.topics
                .borrow_mut()
                .get_mut(&topic.topic_name)
                .unwrap()
                .update(host.clone(), topic);
        }
    }
}
