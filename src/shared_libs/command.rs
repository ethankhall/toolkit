pub mod time_command {
    
    use std::process;

    use clap::ArgMatches;

    use chrono::prelude::*;
    use chrono_tz::{Tz, America};

    use super::super::time::{parse_time_from_array, build_all_time_parse_options};

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
        if args.is_present("example") {
            print_examples()
        } else {
            let input_array: Vec<&str> = args.values_of("INPUT").unwrap().collect();
            match parse_time_from_array(input_array.clone()) {
                Ok(date) => render_output(date),
                Err(input) => {
                    error!("Unable to understand `{}`, please check our know formats with --example", input);
                    process::exit(1);
                }
            }
        }
    }

    fn print_examples() {
        let now = Utc::now().with_timezone(&Tz::UTC);

        println!("Here are all my known formats for `now`.");
        
        for format in vec!["%s%3f", "%s"] { // We understand this format, but don't parse it this way.
            println!("- `{}`", now.format(&format).to_string())
        }

        for format in build_all_time_parse_options() {
            if !format.contains("%#z") { //Input with %#z is undefined and will panic
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
                None => input.format(format)
            };

            println!("{0: >27} || {1:}", text, this_format.to_string());
        }
    }
}