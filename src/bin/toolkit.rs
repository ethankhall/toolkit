#[macro_use]
extern crate clap;
extern crate shared_libs;
extern crate chrono;

use shared_libs::logging::configure_logging;
use shared_libs::command::time_command::do_time_command;

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
            )
        )).get_matches();

    
    configure_logging(
        matches.occurrences_of("debug") as i32,
        matches.is_present("warn"),
        matches.is_present("quite"),
    );

    match matches.subcommand() {
        ("time", Some(time_matches)) => do_time_command(time_matches),
        _           => unreachable!()
    }
}