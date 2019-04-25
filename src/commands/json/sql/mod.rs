mod parser;

use std::io::{BufRead, BufReader, Read};

use clap::ArgMatches;
use parser::{Expression, SqlSource};
use json::JsonValue;

use super::{parse_path, find_field};

use crate::commands::CliError;
use crate::commands::progress::*;

pub fn do_json_sql_command(args: &ArgMatches)-> Result<(), CliError> {

    let exp = Expression::from(args.value_of("EXP").unwrap())?;

    let from_path = match exp.source {
        SqlSource::None => parse_path("."),
        SqlSource::SqlFrom { path } => parse_path(&path)
    };

    let input_paths: Vec<String> = args.values_of("json").unwrap().map(|x| s!(x)).collect();

    let line_processor = LineProcessor { path: from_path };

    for input_path in input_paths.into_iter() {
        let reader = match crate::commands::file::open_file(&input_path) {
            Ok(reader) => BufReader::new(reader),
            Err(e) => {
                error!("Unable to open {} because {}", input_path, e.to_string());
                continue;
            }
        };
        
        line_processor.process_stream(reader);
    }

    Ok(())
}

struct LineProcessor {
    path: Vec<String>,
}

impl LineProcessor {
    fn process_stream<R: Read>(&self, buf_read: BufReader<R>) {
        let mut line_counter = 0;
        for line in buf_read.lines() {
            line_counter += 1;
            match line {
                Ok(line) => {
                    self.parse_json_line(line, line_counter);
                },
                Err(err) => error!("IO error: Line {}: {}", line_counter, err),
            }
        }
    }

    fn parse_json_line(&self, line: String, line_number: u32) {
        match json::parse(&line) {
            Ok(parsed) => {
                if let Some(sub_json) = find_field(&self.path, &parsed) {
                    trace!("sub json: {:?}", sub_json);
                    self.process_json_line(sub_json)
                }
            },
            Err(_) => error!("Line was not JSON: {}", line_number)
        }
    }

    fn process_json_line(&self, parent: &JsonValue) {
        match parent {
            JsonValue::Array(values) => {
                for value in values {
                    if self.filter_obj(&value) {
                        self.print_fields(&value);
                    }
                }
            },
            JsonValue::Object(_) => {
                if self.filter_obj(parent) {
                    self.print_fields(&parent);
                }
            },
            _ => error!("Tried to parse an object that wasn't an array or object, got {:?}", parent)
        }
    }

    fn filter_obj(&self, parent: &JsonValue) -> bool {
        if let JsonValue::Object(obj) = parent {
            true
        } else {
            error!("{:?} is not a json object", parent);
            false
        }
    }

    fn print_fields(&self, parent: &JsonValue) {
        println!("{}", parent.dump());
    }
}