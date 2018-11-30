use chrono::Local;
use std::io::{stderr, stdout};

use fern::Dispatch;
use fern::colors::{Color, ColoredLevelConfig};
use log::Level;

pub fn configure_logging(verbose: i32, warn: bool, quite: bool) {
    let level: Level = if quite {
        log_level(0)
    } else if warn {
        log_level(1)
    } else {
        log_level(verbose + 2)
    };

    let mut dispatch = Dispatch::new();
    if level != Level::Trace || verbose + 2 < 5 {
        dispatch = dispatch
            .level_for("mio", Level::Warn.to_level_filter())
            .level_for("tokio_core", Level::Warn.to_level_filter())
            .level_for("hyper", Level::Warn.to_level_filter());
    }

    let result = configure_logging_output(level, dispatch)
        .level(level.to_level_filter())
        .chain(
            Dispatch::new()
                .filter(|log_meta| Level::Warn <= log_meta.level())
                .chain(stdout()),
        )
        .chain(
            Dispatch::new()
                .filter(|log_meta| Level::Error == log_meta.level())
                .chain(stderr()),
        )
        .apply();

    if result.is_err() {
        panic!("Logger already initialized...");
    }
}

fn log_level(number_of_verbose: i32) -> Level {
    return match number_of_verbose {
        0 => Level::Error,
        1 => Level::Warn,
        2 => Level::Info,
        3 => Level::Debug,
        4 | _ => Level::Trace,
    };
}

fn configure_logging_output(logging_level: Level, dispatch: Dispatch) -> Dispatch {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .warn(Color::Magenta)
        .error(Color::Red)
        .debug(Color::Blue);

    if logging_level == Level::Trace || logging_level == Level::Debug {
        return dispatch.format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Local::now().format("[%Y-%m-%d - %H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        });
    } else {
        return dispatch.format(move |out, message, record| {
            if record.level() == Level::Error {
                out.finish(format_args!(
                    "[{}] {}",
                    colors.color(record.level()),
                    message
                ));
            } else {
                out.finish(format_args!("{}", message));
            }
        });
    }
}
