#![feature(slice_concat_ext)]

extern crate chrono;
extern crate chrono_tz;
extern crate clap;
extern crate fern;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate mime;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate url;

#[macro_export]
macro_rules! s {
    ($x:expr) => {
        $x.to_string()
    };
}

pub mod command;
pub mod logging;
pub mod models;
pub mod output;
pub mod time;
