use std::slice::SliceConcatExt;
use std::str::FromStr;

use clap::ArgMatches;

use chrono::prelude::*;
use chrono_tz::{America, Tz};

use crate::commands::CliError;

static DATE_FORMATS: &'static [&str] = &[
    "%-m/%-d/%y",
    "%Y-%m-%d",
    "%e-%b-%Y",
    "%-m-%-d-%Y",
    "%-m-%-d-%y",
];

static TIME_FORMATS: &'static [&str] = &["%H:%M", "%H:%M:%S", "%I:%M:%S %p", "%I:%M:%S %P"];
static TIMEZONE_FORMATS: &'static [&str] = &["%z", "%:z", "%#z"];
static DATETIME_FORMATS: &'static [&str] = &["%c", "%+", "%Y-%m-%dT%H:%M:%S%:z", "%a %b %d %H:%M:%S %Y"];

static EXPORT_FORMAT: &'static [(&str, &str, Option<Tz>)] = &[
    ("Standard Format in UTC", "%c", None),
    ("Standard Format with Tz", "%+", None),
    ("UNIX EPOCH", "%s", None),
    ("UNIX EPOCH (ms)", "%s%3f", None),
    ("Rendered Format (Offset)", "%a %b %d %H:%M:%S %Z %Y", None),
    (
        "Rendered Format (UTC)",
        "%a %b %d %H:%M:%S %Z %Y",
        Some(Tz::UTC),
    ),
    (
        "Rendered Format (Chicago)",
        "%a %b %d %H:%M:%S %Z %Y",
        Some(America::Chicago),
    ),
    (
        "Rendered Format (LA/SEA)",
        "%a %b %d %H:%M:%S %Z %Y",
        Some(America::Los_Angeles),
    ),
    ("Year-Month-Day", "%Y-%m-%d", None),
    ("Month/Day/Year", "%m/%d/%y", None),
    ("YYYYMMDD", "%Y%m%d", None),
];

pub fn do_time_command(args: &ArgMatches) -> Result<(), CliError> {
    if args.is_present("example") {
        print_examples();
        return Ok(());
    } else {
        let input_array: Vec<&str> = args.values_of("INPUT").unwrap().collect();
        return match parse_time_from_array(input_array.clone()) {
            Ok(date) => Ok(render_output(date)),
            Err(input) => {
                error!(
                    "Unable to understand `{}`, please check our know formats with --example",
                    input
                );
                return Err(CliError::new("Unknown format", 1));
            }
        };
    }
}

fn print_examples() {
    let now = Utc::now().with_timezone(&Tz::UTC);

    println!("Here are all my known formats for `now`.");

    for format in vec!["%s%3f", "%s"] {
        // We understand this format, but don't parse it this way.
        println!("- `{}`", now.format(&format).to_string())
    }

    for format in build_all_time_parse_options() {
        if !format.contains("%#z") {
            //Input with %#z is undefined and will panic
            println!("- `{}`", now.format(&format).to_string())
        }
    }
}

fn render_output(input: DateTime<FixedOffset>) {
    println!("Understood the date was {}", input);
    println!();
    for (text, format, zone) in EXPORT_FORMAT {
        let this_format = match zone {
            Some(zone) => input.with_timezone(zone).format(format),
            None => input.format(format),
        };

        println!("{0: >27} || {1:}", text, this_format.to_string());
    }
}

pub fn build_all_time_parse_options() -> Vec<String> {
    let mut options = Vec::new();
    for datetime_format in DATETIME_FORMATS {
        options.push(s!(datetime_format));
    }

    for date_format in DATE_FORMATS {
        options.push(s!(date_format));

        for time_format in TIME_FORMATS {
            let datetime_format = format!("{} {}", date_format, time_format);
            options.push(datetime_format);

            for timezone_format in TIMEZONE_FORMATS {
                let zoned_datetime_format =
                    format!("{} {} {}", date_format, time_format, timezone_format);

                options.push(zoned_datetime_format);
            }
        }
    }

    return options;
}

pub fn parse_time_from_array(input_array: Vec<&str>) -> Result<DateTime<FixedOffset>, String> {
    let mut local_timezone = Local::now().offset().fix();
    let mut input_string = input_array.join(" ");
    for test in input_array.clone() {
        let zone = match test {
            "PST" => Some(FixedOffset::west(8 * 3600)),
            "PDT" => Some(FixedOffset::west(7 * 3600)),
            _ => match Tz::from_str(test) {
                Ok(data) => Some(Utc::now().with_timezone(&data).offset().fix()),
                Err(_) => None,
            },
        };

        if let Some(tz) = zone {
            local_timezone = tz;
            input_string = input_string.replace(&format!(" {}", test), "");
            break;
        }
    }

    debug!("Input String: {}", input_string);

    return match parse_input(&input_string, local_timezone) {
        Some(value) => Ok(value),
        None => Err(input_string),
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::io::BufRead;
    use std::io::BufReader;
    use std::path::PathBuf;

    #[test]
    fn test_parsing_inputs() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/time_known_formats.txt");

        let f = File::open(d).unwrap();
        let file = BufReader::new(&f);
        for line in file.lines() {
            let l = line.unwrap();
            println!("Validating {} as input.", l);
            let arr = l.split(" ").collect();
            match parse_time_from_array(arr) {
                Ok(_) => {}
                Err(err) => assert!(false, format!("Unable to parse {}", err)),
            }
        }
    }
}

fn parse_input(input: &str, local_timezone: FixedOffset) -> Option<DateTime<FixedOffset>> {
    if "now" == input {
        return Some(Utc::now().with_timezone(&local_timezone));
    }

    if let Ok(result) = input.parse::<i64>() {
        let timestamp = if 15_000_000_000 < result {
            Utc.timestamp(result / 1000, 0)
        } else {
            Utc.timestamp(result, 0)
        };

        return Some(timestamp.with_timezone(&FixedOffset::east(0)));
    }

    if let Ok(parsed) = DateTime::parse_from_rfc3339(input) {
        return Some(parsed);
    }

    if let Ok(parsed) = DateTime::parse_from_rfc2822(input) {
        return Some(parsed);
    }

    for format in build_all_time_parse_options() {
        if let Some(value) = try_convert(input, &format, local_timezone) {
            return Some(value);
        }
    }

    return None;
}

fn try_convert(
    input: &str,
    format: &str,
    local_timezone: FixedOffset,
) -> Option<DateTime<FixedOffset>> {
    match DateTime::parse_from_str(input, format) {
        Ok(value) => return Some(value),
        Err(err) => {
            debug!("Processing {} against {} and got {}", input, format, err);
        }
    }

    match NaiveDateTime::parse_from_str(input, format) {
        Ok(value) => return Some(DateTime::from_utc(value, local_timezone)),
        Err(err) => {
            debug!("Processing {} against {} and got {}", input, format, err);
        }
    }

    match NaiveDate::parse_from_str(input, format) {
        Ok(value) => {
            let midnight = NaiveTime::from_hms_milli(0, 0, 0, 0);
            let date_time = value.and_time(midnight);
            return Some(DateTime::from_utc(date_time, local_timezone));
        }
        Err(err) => {
            debug!("Processing {} against {} and got {}", input, format, err);
        }
    }

    return None;
}
