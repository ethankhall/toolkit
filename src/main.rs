#![feature(slice_concat_ext)]

extern crate chrono;
extern crate colored;
extern crate chrono_tz;
extern crate fern;
#[macro_use]
extern crate serde_derive;
extern crate mime;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate reqwest;
extern crate url;
#[macro_use]
extern crate clap;
extern crate json;
#[macro_use]
extern crate log;
#[macro_use]
extern crate kopy_common_lib;

mod commands;

use clap::App;

use commands::har::exec::do_har_command;
use commands::json::do_json_latest_command;
use commands::time::do_time_command;
use commands::nsq::post::do_send_command;
use commands::nsq::stats::do_status_command;
use kopy_common_lib::configure_logging;

fn main() {
    let yml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yml)
        .version(&*format!("v{}", crate_version!()))
        .get_matches();

    configure_logging(
        matches.occurrences_of("debug") as i32,
        matches.is_present("warn"),
        matches.is_present("quite"),
    );

    let result = match matches.subcommand() {
        ("time", Some(time_matches)) => do_time_command(time_matches),
        ("har", Some(har_matches)) => do_har_command(har_matches),
        ("json", Some(json_matches)) => match json_matches.subcommand() {
            ("latest", Some(filter_matches)) => do_json_latest_command(filter_matches),
            _ => unreachable!(),
        },
        ("nsq", Some(nsq_matches)) => match nsq_matches.subcommand() {
            ("send", Some(send_matches)) => do_send_command(send_matches),
            ("status", Some(send_matches)) => do_status_command(send_matches),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    if let Err(err) = result {
        std::process::exit(err.code);
    }
}
