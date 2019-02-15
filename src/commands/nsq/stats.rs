use std::{thread, time};

use clap::ArgMatches;
use colored::*;

use crate::commands::CliError;

pub fn do_status_command(matches: &ArgMatches) -> Result<(), CliError> {
    let topic = matches.value_of("TOPIC").unwrap();

    let nsq_lookup_host = matches.value_of("nsq_lookup_host").unwrap();
    let nsq_lookup_port = matches.value_of("nsq_lookup_port").unwrap();

    let count = matches.value_of("count").unwrap().parse::<u32>().unwrap();

    let nsq_lookup = format!("{}:{}", nsq_lookup_host, nsq_lookup_port);

    let base_url = match super::api::get_base_url_for_topic(&nsq_lookup, &topic) {
        Some(url) => url,
        None => { return Err(CliError::new("Unable to get NSQ Host", 2)) }
    };
    
    for i in 0..count {
        check_data(&base_url, topic, i);
        let second = time::Duration::from_secs(1);
        thread::sleep(second);
    }

    Ok(())
}

fn check_data(base_url: &str, topic_name: &str, index: u32) {
    let data = match super::api::get_topic_status(base_url, topic_name) {
        Some(data) => data,
        None => {
            warn!("Unable to access NSQ API");
            return;
        }
    };

    for topic in data.topics {
        if index != 0 {
            info!("");
        }
        info!("Current NQS status for {}", topic.topic_name.bold());
        info!("Queue Depth: {}", s!(topic.depth).bold());

        for channel in topic.channels {
            info!("\tChannel {}", channel.channel_name.bold());
            info!("\tDepth: {}", s!(channel.depth).bold());
            info!("\tIn Flight ✈️: {}", s!(channel.in_flight_count).bold());
        }
    }
}