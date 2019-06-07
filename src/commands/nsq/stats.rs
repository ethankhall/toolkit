use std::collections::BTreeMap;
use std::io::{stdout, Stdout, Write};
use std::{thread, time};

use futures::executor::block_on;
use chrono::prelude::*;
use clap::ArgMatches;
use colored::*;
use prettytable::{format, Table};
use termion::screen::*;

use crate::commands::CliError;
use crate::commands::nsq::api::*;

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

pub fn do_stats_command(matches: &ArgMatches) -> Result<(), CliError> {
    let config = ConfigOptions::new(matches);

    let filter = match (matches.values_of("hosts"), matches.values_of("topics")) {
        (Some(hosts), Some(topics)) => {
            NsqFilter::HostAndTopic { hosts: hosts.map(|x| s!(x)).collect(), topics: topics.map(|x| s!(x)).collect() }
        },
        (None, Some(topics)) => {
            NsqFilter::Topic { topics: topics.map(|x| s!(x)).collect() }
        },
        (Some(hosts), None) => {
            NsqFilter::Host { hosts: hosts.map(|x| s!(x)).collect() }
        },
        _ => unimplemented!()
    };

    let state = block_on(NsqState::new(&config.nsq_lookup, filter));

    do_loop(&config, state);
    Ok(())
}

fn do_loop(config: &ConfigOptions, state: NsqState) {
    let mut screen = AlternateScreen::from(stdout());
    let mut counter = 0;
    let mut last_data = None;
    let mut buffer_size: i32 = -1;
    let mut snapshot = state.get_status();

    loop {
        if buffer_size > 0 {
            write!(screen, "{}", termion::cursor::Up(buffer_size as u16),).unwrap();
        }

        let last_buffer_size = print_report(&config, &snapshot, last_data, &mut screen) as i32;
        last_data = Some(snapshot);

        buffer_size = std::cmp::max(buffer_size, last_buffer_size);
        write!(screen, "{}", termion::clear::AfterCursor).unwrap();
        
        let poll_start = Local::now();
        snapshot = block_on(state.update_status());

        let diff = chrono::Duration::seconds(config.delay) - (Local::now() - poll_start);
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

fn print_report(
    config_options: &ConfigOptions,
    current: &NsqSnapshot,
    last_data: Option<NsqSnapshot>,
    screen: &mut AlternateScreen<Stdout>,
) -> usize {
    let mut buffer: Vec<u8> = Vec::new();

    writeln!(buffer, "Polled at {} (UTC: {})", s!(current.pull_finished).bold(), s!(current.pull_finished.with_timezone(&Utc)).bold()).unwrap();

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

fn make_channel_table(config_options: &ConfigOptions, stats: &NsqSnapshot, topic: &str, last: &Option<NsqSnapshot>) -> Option<Table> {
    let mut table = Table::new();

    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![
        "Channel Name",
        "Queue Depth",
        "Queue Depth Change",
        "In Flight ✈️",
        "Total Messages",
    ]);

    let mut channel_written = false;

    for (channel_name, channel) in stats.topics.get(topic).unwrap().consumers.iter() {
        if channel.depth == 0 && config_options.hide_zero_depth {
            continue;
        } else {
            channel_written = true;
        }
        let change = match last {
            Some(last_stats) => match last_stats.get_channel(topic, channel_name) {
                Some(last_channel_stats) => {
                    let difference = (channel.depth as u128 - last_channel_stats.depth as u128) as f64;
                    let mps = difference
                        / (stats.pull_finished - last_stats.pull_finished).num_milliseconds() as f64;
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
            s!(channel.in_progress).bold(),
            s!(channel.finish_count).bold()
        ]);
    }

    if !channel_written {
        return None
    }

    Some(table)
}

fn make_host_table(current: &NsqSnapshot, last: &Option<NsqSnapshot>) -> BTreeMap<String, Table> {
    let mut hosts_table: BTreeMap<String, Table> = BTreeMap::new();

    for (topic_name, details) in current.topics.iter() {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(row!["Host Name", "Depth", "Message Count"]);

        let producers = &details.producers;
        for producer in producers.values() {
            table.add_row(row![
                producer.hostname.bold(),
                s!(producer.depth).bold(),
                s!(producer.message_count).bold()
            ]);
        }

        let aggregate = details.producer_aggregate();

        table.add_row(row![
            "Total".dimmed().yellow(),
            s!(aggregate.depth).dimmed().yellow(),
            s!(aggregate.message_count).dimmed().yellow()
        ]);

        if let Some(ref previous_stats) = last {
            if let Some(last_topic_stats) = previous_stats.topics.get(topic_name) {
                let last_aggregate = last_topic_stats.producer_aggregate();
                let change = aggregate.message_count as u128 - last_aggregate.message_count as u128;
                table.add_row(row![
                    "Change",
                    "",
                    change
                ]);
                let mps = change as f64;
                let mps =
                    mps / (current.pull_finished - previous_stats.pull_finished).num_milliseconds() as f64;
                let mps = mps * 1000 as f64;
                table.add_row(row!["Rate", "", format!("{:.2} m/s", mps)]);
            }
        }

        hosts_table.insert(s!(topic_name), table);
    }

    hosts_table
}