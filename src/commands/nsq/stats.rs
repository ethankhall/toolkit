use std::collections::BTreeMap;
use std::io::{stdout, Stdout, Write};
use std::sync::Mutex;
use std::{thread, time};

use chrono::prelude::*;
use clap::ArgMatches;
use colored::*;
use prettytable::{format, Table};
use termion::screen::*;

use crate::commands::nsq::api::*;
use crate::commands::CliError;

lazy_static! {
    static ref TOPIC_URLS: Mutex<Vec<TopicUrlElement>> = Mutex::new(Vec::new());
}

struct TopicUrlElement {
    name: String,
    urls: Vec<String>,
}

pub fn do_status_command(matches: &ArgMatches) -> Result<(), CliError> {
    let topics = matches
        .values_of("TOPIC")
        .map(|x| x.collect())
        .unwrap_or_else(|| vec![]);

    let nsq_lookup_host = matches.value_of("nsq_lookup_host").unwrap();
    let nsq_lookup_port = matches.value_of("nsq_lookup_port").unwrap();

    let count = matches.value_of("count").map(|x| x.parse::<u32>().unwrap());
    let mut delay = matches
        .value_of("delay")
        .map(|x| x.parse::<i64>().unwrap())
        .unwrap();

    if delay < 1 {
        warn!("Delay was less than 1, defaulting to 1");
        delay = 1;
    }

    let nsq_lookup = format!("{}:{}", nsq_lookup_host, nsq_lookup_port);

    for topic in topics {
        let base_urls = super::api::get_base_url_for_topic(&nsq_lookup, &topic);
        if base_urls.is_empty() {
            return Err(CliError::new("Unable to get NSQ Host", 2));
        }

        TOPIC_URLS.lock().unwrap().push(TopicUrlElement {
            name: s!(topic),
            urls: base_urls,
        });
    }

    let mut screen = AlternateScreen::from(stdout());
    let mut counter = 0;
    let mut last_data = None;
    let mut buffer_size: i32 = -1;

    loop {
        let calculated = find_data();
        if buffer_size > 0 {
            write!(
                screen,
                "{}",
                termion::cursor::Up(buffer_size as u16),
            )
            .unwrap();
        }
        buffer_size = print_report(&calculated, last_data, &mut screen) as i32;

        let diff = chrono::Duration::seconds(delay) - (Local::now() - calculated.poll_time());
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

fn find_data() -> NsqStats {
    let mut stats = NsqStats::new();

    for topic in TOPIC_URLS.lock().unwrap().iter() {
        for base_url in topic.urls.iter() {
            let data = super::api::get_topic_status(&base_url, &topic.name);
            stats.register(&base_url, data)
        }
    }

    stats
}

fn print_report(
    current: &NsqStats,
    last_data: Option<NsqStats>,
    screen: &mut AlternateScreen<Stdout>,
) -> usize {
    let mut buffer: Vec<u8> = Vec::new();

    writeln!(buffer, "Polled at {}", s!(current.poll_time).bold()).unwrap();

    for (topic_name, host_table) in make_host_table(&current, &last_data) {
        writeln!(buffer, "\n📇 {}", topic_name.bold()).unwrap();

        host_table.print(&mut buffer).unwrap();

        writeln!(buffer, "").unwrap();
        make_channel_table(&current, &topic_name, &last_data)
            .print(&mut buffer)
            .unwrap();
    }

    let mut lines: usize = 0;
    let line_buffer = buffer.split(|x| x == &('\n' as u8));
    for line in line_buffer {
        lines += 1;
        writeln!(
            screen,
            "{}{}",
            String::from_utf8(line.to_vec()).unwrap(),
            termion::clear::UntilNewline
        )
        .unwrap();
    }

    lines
}

fn make_channel_table(stats: &NsqStats, topic: &str, last: &Option<NsqStats>) -> Table {
    let mut table = Table::new();

    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![
        "Channel Name",
        "Queue Depth",
        "Queue Depth Change",
        "In Flight ✈️"
    ]);

    for (channel_name, channel) in stats.topics.get(topic).unwrap().channels.iter() {
        let change = match last {
            Some(last_stats) => match last_stats.get_channel(topic, channel_name) {
                Some(last_channel_stats) => {
                    let difference = (last_channel_stats.depth - channel.depth) as f64;
                    let mps = difference
                        / (last_stats.poll_time - stats.poll_time).num_milliseconds() as f64;
                    let mps = mps * 1000 as f64;

                    format!("{} ({:.2} m/s)", difference, mps)
                }
                None => s!("0"),
            },
            None => s!("0"),
        };

        table.add_row(row![
            channel_name.bold(),
            s!(channel.depth).bold(),
            change.bold(),
            s!(channel.in_flight_count).bold()
        ]);
    }

    table
}

fn make_host_table(current: &NsqStats, last: &Option<NsqStats>) -> BTreeMap<String, Table> {
    let mut hosts_table: BTreeMap<String, Table> = BTreeMap::new();

    for (topic_name, details) in current.topics.iter() {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(row!["Host Name", "Depth", "Message Count"]);

        let hosts = &details.hosts;
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
            if let Some(last_topic_stats) = previous_stats.topics.get(topic_name) {
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
                table.add_row(row!["Rate", "", format!("{:.2} m/s", mps)]);
            }
        }

        hosts_table.insert(s!(topic_name), table);
    }

    hosts_table
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct TopicMetrics {
    depth: i32,
    message_count: i32,
}

#[derive(Debug, Clone)]
struct TopicStats {
    channels: BTreeMap<String, ChannelMetrics>,
    hosts: BTreeMap<String, TopicMetrics>,
    total_depth: i32,
    total_message_count: i32,
}

impl TopicStats {
    fn new() -> Self {
        return TopicStats {
            channels: BTreeMap::new(),
            hosts: BTreeMap::new(),
            total_depth: 0,
            total_message_count: 0,
        };
    }

    fn update(&mut self, host: String, details: TopicDetails) {
        let topic_metrics = TopicMetrics {
            depth: details.depth,
            message_count: details.message_count,
        };
        self.hosts.insert(host, topic_metrics);

        self.total_depth += details.depth;
        self.total_message_count += details.message_count;

        for channel in details.channels {
            if !self.channels.contains_key(&channel.channel_name) {
                self.channels.insert(
                    channel.channel_name.clone(),
                    ChannelMetrics {
                        depth: 0,
                        in_flight_count: 0,
                    },
                );
            }

            let channel_value = self.channels.get_mut(&channel.channel_name).unwrap();
            channel_value.update(channel.depth, channel.in_flight_count);
        }
    }
}

struct NsqStats {
    poll_time: DateTime<Local>,
    topics: BTreeMap<String, TopicStats>,
}

impl NsqStats {
    fn new() -> Self {
        return NsqStats {
            poll_time: Local::now(),
            topics: BTreeMap::new(),
        };
    }

    fn poll_time(&self) -> DateTime<Local> {
        self.poll_time.clone()
    }

    fn get_channel(&self, topic_name: &str, channel_name: &str) -> Option<&ChannelMetrics> {
        self.topics
            .get(topic_name)
            .and_then(|x| x.channels.get(channel_name))
    }

    fn register(&mut self, url: &str, details: Option<StatusTopicsDetails>) {
        let host = url::Url::parse(url).unwrap().host().unwrap().to_string();
        let details = match details {
            None => {
                warn!("Unable to access NSQ API for {}", host.clone());
                return;
            }
            Some(details) => details,
        };

        for topic in details.topics {
            if !self.topics.contains_key(&topic.topic_name) {
                self.topics
                    .insert(topic.topic_name.clone(), TopicStats::new());
            }

            self.topics
                .get_mut(&topic.topic_name)
                .unwrap()
                .update(host.clone(), topic);
        }
    }
}
