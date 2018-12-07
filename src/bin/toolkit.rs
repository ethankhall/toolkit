#[macro_use]
extern crate clap;
extern crate shared_libs;
extern crate chrono;

use shared_libs::logging::configure_logging;
use shared_libs::command::time_command::do_time_command;
use shared_libs::command::har_command::do_har_command;

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
            (about: "Har...dy up those the hatches!")
            (long_about: "Take a Har file, apply some filtering, then output a new Har file")
            (@arg filter_domain: --("filter-domain") +takes_value +multiple +require_equals "Include requests for specificed domain")
            (@arg filter_path: --("filter-path") +takes_value +require_equals "A Regex to filter the path on")
            (@arg filter_context_type: --("filter-content-type") +takes_value +multiple +require_equals "Include request that respond with specific types")
            (@arg output: -o --output +takes_value +require_equals "Output to a file instead of stdout")
            (@arg output_format: --format +takes_value default_value[har] possible_value[har html md markdown] "Instead of output")
            (@arg INPUT: +takes_value +required "Input to be parsed.")
            )
        ).get_matches();

    
    configure_logging(
        matches.occurrences_of("debug") as i32,
        matches.is_present("warn"),
        matches.is_present("quite"),
    );

    let result = match matches.subcommand() {
        ("time", Some(time_matches)) => do_time_command(time_matches),
        ("har", Some(time_matches)) => do_har_command(time_matches),
        _           => unreachable!()
    };

    if let Err(code) = result {
        std::process::exit(code);
    }
}