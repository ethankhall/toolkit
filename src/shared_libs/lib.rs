#![feature(slice_concat_ext)]

extern crate chrono;
extern crate chrono_tz;
extern crate fern;
extern crate clap;
#[macro_use]
extern crate log;

#[macro_export]
macro_rules! s {
    ($x:expr) => {
        $x.to_string()
    };
}

pub mod logging;
pub mod command;
pub mod time;