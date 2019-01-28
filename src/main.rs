#![feature(slice_concat_ext)]

extern crate chrono;
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
use commands::har::exec::do_har_command;
use commands::json::do_json_filter_command;
use commands::time::do_time_command;
use commands::nsq::post::do_send_command;
use kopy_common_lib::configure_logging;

fn main() {
    let matches = clap_app!(MyApp =>
        (@setting SubcommandRequiredElseHelp)
        (version: crate_version!())
        (about: "Toolkit of useful dev tools")
        (@group logging =>
                (@arg debug: -d --debug ... +global "Turn debugging information on")
                (@arg quite: -q --quite +global "Only error output will be displayed")
                (@arg warn: -w --warn +global "Only error output will be displayed")
        )
        (@subcommand time =>
            (about: "Takes a time, and gives a bunch of details about it")
            (@group options =>
                (@attributes +required)
                (@arg example: --example "Output the current time in all understood formats")
                (@arg INPUT: +takes_value ... "Input to be parsed, will be merged into a single string")
            ))
        (@subcommand har =>
            (about: "Har...dy up those the matches!")
            (long_about: "Take a Har file, apply some filtering, then output a new Har file")
            (@arg filter_domain: --("filter-domain") +takes_value +multiple +require_equals "Include requests for specificed domain")
            (@arg filter_path: --("filter-path") +takes_value +multiple +require_equals "A Regex to filter the path on")
            (@arg filter_context_type: --("filter-content-type") +takes_value +multiple +require_equals "Include request that respond with specific types")
            (@arg output: -o --output +takes_value +require_equals "Output to a file instead of stdout")
            (@arg output_format: --format +takes_value default_value[har] possible_value[har html md markdown] "Instead of output")
            (@arg INPUT: +takes_value +required "Input to be parsed.")
            )
        (@subcommand nsq =>
            (@setting SubcommandRequiredElseHelp)
            (@subcommand send =>
                (@setting ArgRequiredElseHelp)
                (about: "Send a \\n deliminated file to an NSQ topic")
                (@arg offset: --offset +takes_value "Where in the file to start posting.")
                (@arg limit: --limit +takes_value "Limit the number of posts we send")
                (@arg rate: --rate +takes_value "Limit the rate we send posts. Defaults to 200 m/s")
                (@arg nsq_lookup_host: --("lookup-host") +required +takes_value "Host to the NSQ Lookup host")
                (@arg nsq_lookup_port: --("lookup-port") +required +takes_value "HTTP port to NSQ Lookup service")
                (@arg TOPIC: +required +takes_value "Which topic should be posted to")
                (@arg INPUT: +required +takes_value "File to post line by line to the Bus")
            ))
        (@subcommand json =>
            (@setting SubcommandRequiredElseHelp)
            (about: "Nifty JSON command(s)")
            (@subcommand filter =>
               (about: "Filter new-line delemited JSON stream to the newest message")
                (long_about: "If a JSON blob has both an ID that's unique, and a timestamp/version field. Filter the stream for the latest ID/version field.")
                (@arg id: -i --("id-path") +required +takes_value "A field like a ID or GUID that will be unique between different logical units, but the same for the same unit at different times.")
                (@arg seq: -p --("sequence-path") +required +takes_value "Path to a value that will be greater than a previous value, based on order the the blob was created.")
                (@arg OUTPUT: +required +takes_value "File to write output to")
                )
            )
        ).get_matches();

    configure_logging(
        matches.occurrences_of("debug") as i32,
        matches.is_present("warn"),
        matches.is_present("quite"),
    );

    let result = match matches.subcommand() {
        ("time", Some(time_matches)) => do_time_command(time_matches),
        ("har", Some(har_matches)) => do_har_command(har_matches),
        ("json", Some(json_matches)) => match json_matches.subcommand() {
            ("filter", Some(filter_matches)) => do_json_filter_command(filter_matches),
            _ => unreachable!(),
        },
        ("nsq", Some(nsq_matches)) => match nsq_matches.subcommand() {
            ("send", Some(send_matches)) => do_send_command(send_matches),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    if let Err(err) = result {
        std::process::exit(err.code);
    }
}
