use std::collections::BTreeMap;
use std::io::{stdout, Stdout, Write};
use std::sync::Mutex;
use std::{thread, time};
use futures::executor::block_on;

use chrono::prelude::*;
use clap::ArgMatches;
use colored::*;
use prettytable::{format, Table};
use termion::screen::*;

use super::TopicUrlElement;
use crate::commands::nsq::api::*;
use crate::commands::CliError;
use crate::commands::progress::*;

lazy_static! {
    static ref TOPIC_URLS: Mutex<Vec<TopicUrlElement>> = Mutex::new(Vec::new());
}

struct ConfigOptions {
    nsq_lookup: String,
    delay: i64,
    count: Option<u32>,
    hide_hosts: bool,
    hide_zero_depth: bool,
}

impl ConfigOptions {
    fn new(matches: &ArgMatches) -> Self {

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
        let hide_hosts = matches.is_present("hide_hosts");
        let hide_zero_depth = matches.is_present("hide_zero_depth");

        ConfigOptions {
            nsq_lookup,
            delay,
            count,
            hide_hosts,
            hide_zero_depth,
        }
    }
}

pub fn do_host_status_command(matches: &ArgMatches) -> Result<(), CliError> {
    let hosts = matches
        .values_of("HOSTS")
        .map(|x| x.collect())
        .unwrap_or_else(|| vec![]);

    let config = ConfigOptions::new(matches);

    match super::api::get_base_url_for_hosts(&config.nsq_lookup, &hosts) {
        Some(mut element) => TOPIC_URLS.lock().unwrap().append(&mut element),
        None => {
            return Err(CliError::new("Unable to get NSQ Host", 2));
        }
    }

    do_printing(&config);

    return Ok(())
}

pub fn do_topic_status_command(matches: &ArgMatches) -> Result<(), CliError> {
    let topics = matches
        .values_of("TOPIC")
        .map(|x| x.collect())
        .unwrap_or_else(|| vec![]);

    let config = ConfigOptions::new(matches);

    for topic in topics {
        match super::api::get_base_url_for_topic(&config.nsq_lookup, &topic) {
            Some(element) => TOPIC_URLS.lock().unwrap().push(element),
            None => {
                return Err(CliError::new("Unable to get NSQ Host", 2));
            }
        }
    }

    do_printing(&config);

    Ok(())
}

fn do_printing(config: &ConfigOptions) {
    let mut screen = AlternateScreen::from(stdout());
    let mut counter = 0;
    let mut last_data = None;
    let mut buffer_size: i32 = -1;

    loop {
        let calculated = find_data();
        if buffer_size > 0 {
            write!(screen, "{}", termion::cursor::Up(buffer_size as u16),).unwrap();
        }
        let last_buffer_size = print_report(&config, &calculated, last_data, &mut screen) as i32;

        buffer_size = std::cmp::max(buffer_size, last_buffer_size);
        write!(screen, "{}", termion::clear::AfterCursor).unwrap();

        let diff = chrono::Duration::seconds(config.delay) - (Local::now() - calculated.poll_time());
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

        if let Some(limit) = config.count {
            if counter >= limit {
                break;
            }
        }
    }
}

fn find_data() -> NsqStats {
    let mut stats = NsqStats::new();

    let lock = TOPIC_URLS.lock().unwrap();
    let mut process_queue = Vec::new();

    for topic in lock.iter() {
        for base_url in topic.urls.iter() {
            process_queue.push(get_topic_status(&base_url, &topic.name));
        }
    }

    let pb = ProgressBarHelper::new(ProgressBarType::SizedProgressBar(process_queue.len(), "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} Fetching topics..."));

    for item in process_queue {
        pb.inc();
        let (base_url, data) = block_on(item);
        stats.register(&base_url, data);
    }

    stats
}

async fn get_topic_status<'a>(base_url: &'a str, topic: &'a str) -> (String, Option<StatusTopicsDetails>) {
    (s!(base_url), super::api::get_topic_status(&base_url, &topic))
}

fn print_report(
    config_options: &ConfigOptions,
    current: &NsqStats,
    last_data: Option<NsqStats>,
    screen: &mut AlternateScreen<Stdout>,
) -> usize {
    let mut buffer: Vec<u8> = Vec::new();

    writeln!(buffer, "Polled at {}", s!(current.poll_time).bold()).unwrap();

    for (topic_name, host_table) in make_host_table(&current, &last_data) {
        writeln!(buffer, "\n📇 {}", topic_name.bold()).unwrap();
        
        if !config_options.hide_hosts {
            host_table.print(&mut buffer).unwrap();
        }

        if let Some(table) = make_channel_table(&config_options, &current, &topic_name, &last_data) {
            writeln!(buffer, "").unwrap();
            table
                .print(&mut buffer)
                .unwrap();
        }
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

fn make_channel_table(config_options: &ConfigOptions, stats: &NsqStats, topic: &str, last: &Option<NsqStats>) -> Option<Table> {
    let mut table = Table::new();

    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![
        "Channel Name",
        "Queue Depth",
        "Queue Depth Change",
        "In Flight ✈️"
    ]);

    let mut channel_written = false;

    for (channel_name, channel) in stats.topics.get(topic).unwrap().channels.iter() {
        if channel.depth == 0 && config_options.hide_zero_depth {
            continue;
        } else {
            channel_written = true;
        }
        let change = match last {
            Some(last_stats) => match last_stats.get_channel(topic, channel_name) {
                Some(last_channel_stats) => {
                    let difference = (channel.depth - last_channel_stats.depth) as f64;
                    let mps = difference
                        / (stats.poll_time - last_stats.poll_time).num_milliseconds() as f64;
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

    if !channel_written {
        return None
    }

    Some(table)
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
                let change = details.total_message_count as i128 - last_topic_stats.total_message_count as i128;
                table.add_row(row![
                    "Change",
                    "",
                    change
                ]);
                let mps = change as f64;
                let mps =
                    mps / (current.poll_time - previous_stats.poll_time).num_milliseconds() as f64;
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
    depth: i128,
    in_flight_count: i128,
}

impl ChannelMetrics {
    fn update(&mut self, depth: u64, in_flight_count: u64) {
        self.depth += depth as i128;
        self.in_flight_count += in_flight_count as i128;
    }
}

#[derive(Debug, Clone)]
struct TopicMetrics {
    depth: u64,
    message_count: u64,
}

#[derive(Debug, Clone)]
struct TopicStats {
    channels: BTreeMap<String, ChannelMetrics>,
    hosts: BTreeMap<String, TopicMetrics>,
    total_depth: u64,
    total_message_count: u64,
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
