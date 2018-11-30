pub mod time {
    
    use std::slice::SliceConcatExt;
    use std::process;
    use std::str::FromStr;

    use clap::ArgMatches;

    use chrono::prelude::*;
    use chrono_tz::{Tz, America};

    static DATE_FORMATS: &'static [&str] = &[
        "%-m/%-d/%y",
        "%Y-%m-%d",
        "%e-%b-%Y",
        "%-m-%-d-%Y",
        "%-m-%-d-%y"
    ];

    static TIME_FORMATS: &'static [&str] = &[
        "%H:%M",
        "%H:%M:%S",
        "%I:%M:%S %p",
        "%I:%M:%S %P"
    ];

    static DATETIME_FORMATS: &'static [&str] = &[
        "%c",
        "%+",
        "%a %b %d %H:%M:%S %Y"
    ];

    static EXPORT_FORMAT: &'static [(&str, &str, Option<Tz>)] = & [
        ("Standard Format in UTC", "%c", None),
        ("Standard Format with Tz", "%+", None),
        ("UNIX EPOCH", "%s", None),
        ("UNIX EPOCH (ms)", "%s%3f", None),
        ("Rendered Format (Orig)", "%a %b %d %H:%M:%S %Z %Y", None),
        ("Rendered Format (UTC)", "%a %b %d %H:%M:%S %Z %Y", Some(Tz::UTC)),
        ("Rendered Format (Chicago)", "%a %b %d %H:%M:%S %Z %Y", Some(America::Chicago)),
        ("Rendered Format (LA/SEA)", "%a %b %d %H:%M:%S %Z %Y", Some(America::Los_Angeles)),
        ("Year-Month-Day", "%Y-%m-%d", None),
        ("Month/Day/Year", "%m/%d/%y", None),
        ("YYYYMMDD", "%Y%m%d", None),
    ];

    pub fn do_time_command(args: &ArgMatches) {
        let input_array: Vec<&str> = args.values_of("INPUT").unwrap().collect();

        let mut local_timezone = Local::now().offset().fix();
        let mut input_string = input_array.join(" ");
        for test in input_array.clone() {

            let zone = match test {
                "PST" => Some(FixedOffset::west(8 * 3600)),
                "PDT" => Some(FixedOffset::west(7 * 3600)),
                _ => {
                    match Tz::from_str(test) {
                        Ok(data) => Some(Utc::now().with_timezone(&data).offset().fix()),
                        Err(_) => None
                    }
                }
            };

            if let Some(tz) = zone {
                local_timezone = tz;
                input_string = input_string.replace(&format!(" {}", test), "");
                break;
            }
        }

        debug!("Input String: {}", input_string);

        match parse_input(&input_string, local_timezone) {
            Some(date) => render_output(date),
            None => {
                error!("Unable to understand `{}`, please check our know formats with --example", input_string);
                process::exit(1);
            }
        }
    }

    fn render_output(input: DateTime<FixedOffset>) {
        println!("Understood the date was {}", input);
        println!();
        for (text, format, zone) in EXPORT_FORMAT {
            let this_format = match zone {
                Some(zone) => input.with_timezone(zone).format(format),
                None => input.format(format)
            };

            println!("{0: >27} || {1:}", text, this_format.to_string());
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

        // Try full date-time formats
        for datetime_format in DATETIME_FORMATS {
            match DateTime::parse_from_str(input, datetime_format) {
                Ok(value) => return Some(value),
                Err(err) => {
                    debug!("Processing {} against {} and got {}", input, datetime_format, err);
                }
            }

            match NaiveDateTime::parse_from_str(input, datetime_format) {
                Ok(value) => return Some(DateTime::from_utc(value, local_timezone)),
                Err(err) => {
                    debug!("Processing {} against {} and got {}", input, datetime_format, err);
                }
            }
        }

        for date_format in DATE_FORMATS {
            if let Ok(value) = NaiveDate::parse_from_str(input, date_format) {
                return Some(DateTime::from_utc(value.and_hms(0, 0, 0), local_timezone));
            }

            for time_format in TIME_FORMATS {
                let datetime_format = format!("{} {}", date_format, time_format);

                match DateTime::parse_from_str(input, &datetime_format) {
                    Ok(value) => return Some(value),
                    Err(err) => {
                        debug!("Processing {} against {} and got {}", input, datetime_format, err);
                    }
                }

                match NaiveDateTime::parse_from_str(input, &datetime_format) {
                    Ok(value) => return Some(DateTime::from_utc(value, local_timezone)),
                    Err(err) => {
                        debug!("Processing {} against {} and got {}", input, datetime_format, err);
                    }
                }
            }
        }

        return None;
    }
}