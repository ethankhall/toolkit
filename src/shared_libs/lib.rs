#![feature(slice_concat_ext)]

extern crate chrono;
extern crate chrono_tz;
extern crate fern;
extern crate clap;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate regex;
extern crate mime;


#[macro_export]
macro_rules! s {
    ($x:expr) => {
        $x.to_string()
    };
}

pub mod logging;
pub mod command;
pub mod time;
pub mod models;
pub mod output;