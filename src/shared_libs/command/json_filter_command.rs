use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead, LineWriter};
use std::path::PathBuf;

use clap::ArgMatches;
use indicatif::{ProgressBar, ProgressStyle};
use json::JsonValue;

#[derive(Debug)]
struct Record {
    id: String,
    version: i32,
    data: String,
}

pub fn do_json_filter_command(args: &ArgMatches) -> Result<(), i32>{
    let output_path = args.value_of("OUTPUT").unwrap();
    let output_path = PathBuf::from(output_path);

    let id_path: Vec<&str> = args
        .value_of("ID_PATH")
        .unwrap()
        .split_terminator(".")
        .collect();
    let version_path: Vec<&str> = args
        .value_of("VESION_PATH")
        .unwrap()
        .split_terminator(".")
        .collect();

    let file = File::create(output_path).unwrap();
    let mut file = LineWriter::new(file);

    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.dim} {spinner} {wide_msg}");
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(spinner_style.clone());

    let mut records: BTreeMap<String, Record> = BTreeMap::new();

    let mut counter = 0;
    for line in io::stdin().lock().lines() {
        match line {
            Ok(line) => {
                counter += 1;
                if counter % 10 == 0 {
                    progress_bar.set_message(&format!(
                        "Reading line {}\t Used: {}",
                        counter,
                        records.len()
                    ));
                }

                let json_line = json::parse(&line).unwrap();

                let id = match find_field(&id_path, &json_line) {
                    Some(value) => value.as_str().unwrap(),
                    None => {
                        warn!("Skipping `{}` because id was missing.", &line);
                        continue;
                    }
                };

                let version = match find_field(&version_path, &json_line) {
                    Some(value) => value.as_i32().unwrap(),
                    None => {
                        warn!("Skipping `{}` because version was missing.", &line);
                        continue;
                    }
                };

                let record = Record {
                    id: id.to_string(),
                    version: version,
                    data: line,
                };

                match records.get(id) {
                    Some(row) if row.version > version => continue,
                    _ => records.insert(id.to_string(), record),
                };
            }
            Err(err) => error!("IO error: Line {}: {}", counter, err),
        }
    }

    let mut write_counter = 0;

    for record in records.values() {
        write_counter += 1;
        progress_bar.set_message(&format!(
            "Writing line {} of {}",
            write_counter,
            records.len()
        ));
        let line = format!("{}\n", record.data);
        if file.write_all(line.as_bytes()).is_err() {
            error!("Trouble writing line {} to disk", write_counter);
        }
    }

    return Ok(());
}

fn find_field<'a>(field: &Vec<&str>, json_input: &'a JsonValue) -> Option<&'a JsonValue> {
    let mut value = json_input;

    for part in field {
        let part = part.to_string();
        value = &value[&part];
        if value.is_null() {
            return None;
        }
    }

    return Some(value);
}
