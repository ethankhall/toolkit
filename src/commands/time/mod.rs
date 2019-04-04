use std::slice::SliceConcatExt;

mod parse;

use clap::ArgMatches;

use colored::*;
use chrono::prelude::*;

use parse::TimeResult;
use crate::commands::CliError;

pub fn do_time_command(args: &ArgMatches) -> Result<(), CliError> {
    let input_array: Vec<&str> = args.values_of("INPUT").unwrap().collect();
    let input_string = input_array.join(" ");
    return match (
        parse::parse(&input_string),
        args.is_present("utc_only"),
    ) {
        (Ok(date), true) => render_utc(date),
        (Ok(date), false) => render_full_output(date),
        (Err(_), _) => {
                error!(
                    "Unable to understand `{}`",
                    input_string
                );
                return Err(CliError::new("Unknown format", 1));
            }
    };
}


fn render_full_output(input: TimeResult) -> Result<(), CliError> {
    let datetimes = match input {
        TimeResult::Epoch(epoch) => {
            epoch.make_permutations()
        },
        TimeResult::String(string_format) => {
            string_format.make_permutations()
        }
    };

    let mut first = true;
    for datetime in datetimes {
        if !first {
            println!();
        }
        first = false;

        println!("Understood the date was {}", format!("{}", datetime).bold());
        println!("  ├── Date 'human'': {}", format!("{}", datetime.format("%b %e %T %Y")).bold());
        println!("  ├── Date in M/D/Y: {}", format!("{}/{}/{}", datetime.month(), datetime.day(), datetime.year()).bold());
        println!("  ├── Date in YMD: {}", format!("{}{:02}{:02}", datetime.year(), datetime.month(), datetime.day()).bold());
        println!("  ├── Day in year: {}", format!("{}", datetime.ordinal()).bold());
        println!("  ├── ISO week {}", format!("{}-{}", datetime.iso_week().year(), datetime.iso_week().week()).bold());
        println!("  ├── Day of week: {}", format!("{:?}", datetime.weekday()).bold());
        println!("  └── Time: {}", format!("{} {}", datetime.time(), datetime.timezone()).bold());
        println!("    ├── Unix epoch(s):  {}", format!("{}", datetime.timestamp()).bold());
        println!("    ├── Unix epoch(ms): {}", format!("{}", datetime.timestamp_millis()).bold());
        println!("    ├── Unix epoch(ns): {}", format!("{}", datetime.timestamp_nanos()).bold());
        println!("    ├── In UTC: {}", format!("{}", datetime.with_timezone(&chrono::Utc)).bold());
        println!("    ├── In Eastern: {}", format!("{}", datetime.with_timezone(&chrono_tz::US::Eastern)).bold());
        println!("    ├── In Central: {}", format!("{}", datetime.with_timezone(&chrono_tz::US::Central)).bold());
        println!("    ├── In Mountain: {}", format!("{}", datetime.with_timezone(&chrono_tz::US::Mountain)).bold());
        println!("    ├── In Arazona: {}", format!("{}", datetime.with_timezone(&chrono_tz::US::Arizona)).bold());
        println!("    └── In Pacific: {}", format!("{}", datetime.with_timezone(&chrono_tz::US::Pacific)).bold());
    }

    Ok(())
}

fn render_utc(input: TimeResult) -> Result<(), CliError> {
    let datetime = match input {
        TimeResult::Epoch(epoch) => {
            epoch.to_utc_date_time()
        },
        TimeResult::String(string_format) => {
            string_format.to_utc_date_time()
        }
    };

    println!("{}", datetime);
    Ok(())
}
