extern crate chrono;
extern crate chrono_tz;
extern crate colored;
extern crate fern;
#[macro_use]
extern crate serde_derive;
extern crate mime;
extern crate regex;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate url;
#[macro_use]
extern crate clap;
extern crate json;
#[macro_use]
extern crate log;
#[macro_use]
extern crate kopy_common_lib;
#[macro_use]
extern crate prettytable;
extern crate crossterm;
#[macro_use]
extern crate lazy_static;
extern crate atty;
extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate futures;
extern crate futures_cpupool;
extern crate itertools;

use std::sync::Mutex;

mod commands;

use clap::App;

use commands::har::exec::do_har_command;
use commands::json::*;
use commands::nsq::post::do_send_command;
use commands::nsq::stats::do_stats_command;
use commands::time::do_time_command;
use kopy_common_lib::configure_logging;

lazy_static! {
    static ref DEBUG_LEVEL: Mutex<i32> = Mutex::new(0);
}

fn main() {
    let yml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yml)
        .version(&*format!("v{}", crate_version!()))
        .get_matches();

    let debug_level = matches.occurrences_of("debug") as i32;
    {
        *DEBUG_LEVEL.lock().unwrap() = debug_level;
    }

    configure_logging(
        debug_level,
        matches.is_present("warn"),
        matches.is_present("quite"),
    );

    let result = match matches.subcommand() {
        ("time", Some(time_matches)) => do_time_command(time_matches),
        ("har", Some(har_matches)) => do_har_command(har_matches),
        ("json", Some(json_matches)) => match json_matches.subcommand() {
            ("latest", Some(filter_matches)) => do_json_latest_command(filter_matches),
            ("sql", Some(filter_matches)) => do_json_sql_command(filter_matches),
            _ => unreachable!(),
        },
        ("nsq", Some(nsq_matches)) => match nsq_matches.subcommand() {
            ("send", Some(send_matches)) => do_send_command(send_matches),
            ("stats", Some(send_matches)) => do_stats_command(send_matches),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    if let Err(err) = result {
        std::process::exit(err.code);
    }
}
