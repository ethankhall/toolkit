use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufRead, BufReader, LineWriter};
use std::path::PathBuf;

use clap::ArgMatches;
use json::JsonValue;

use crate::commands::CliError;
use crate::commands::progress::*;

#[derive(Debug)]
struct Record {
    id: String,
    version: i32,
    data: String,
}

#[test]
fn leading_dot_will_be_ignored() {
    let split: Vec<String> = parse_path(".abc.123");

    assert_eq!(2, split.len());
}

fn parse_path(path: &str) -> Vec<String> {
    let path = if path.starts_with(".") {
        path.replacen(".", "", 1)
    } else {
        path.to_string()
    };

    let split: Vec<String> = path.split_terminator(".").map(|x| x.to_string()).collect();

    return split;
}

fn build_key(keys: &Vec<Vec<String>>, json_input: &JsonValue) -> Option<String> {
    let mut key_list: Vec<String> = Vec::new();

    for part in keys {
        if let Some(key) = find_field(part, json_input) {
            key_list.push(s!(key.as_str().unwrap_or("null")));
        }
    }

    if key_list.is_empty() {
        return None;
    }

    return Some(key_list.join(":"));
}

pub fn do_json_latest_command(args: &ArgMatches) -> Result<(), CliError> {
    let output_path = args.value_of("OUTPUT").unwrap();
    let output_path = PathBuf::from(output_path);

    let id_fields: Vec<Vec<String>> = args
        .values_of("id")
        .unwrap()
        .into_iter()
        .map(|x| parse_path(x))
        .collect();

    let version_path: Vec<String> = parse_path(args.value_of("seq").unwrap());

    let file = File::create(output_path).unwrap();
    let mut file = LineWriter::new(file);

    let pb = ProgressBarHelper::new(ProgressBarType::UnsizedProgressBar("{prefix:.bold.dim} {spinner:.green} {wide_msg}"));

    let mut records: BTreeMap<String, Record> = BTreeMap::new();

    let input_paths: Vec<String> = args.values_of("INPUT").unwrap().map(|x| s!(x)).collect();

    let mut counter = 0;
    for input_path in input_paths.into_iter() {
        let reader = match crate::commands::file::open_file(&input_path) {
            Ok(reader) => BufReader::new(reader),
            Err(e) => {
                error!("Unable to open {} because {}", input_path, e.to_string());
                continue;
            }
        };

        for line in reader.lines() {
            match line {
                Ok(line) => {
                    counter += 1;
                    if counter % 10 == 0 {
                        pb.set_message(&format!(
                            "Reading line {}\t Used: {}",
                            counter,
                            records.len()
                        ));
                    }

                    let json_line = json::parse(&line).unwrap();

                    let id = match build_key(&id_fields, &json_line) {
                        Some(value) => value,
                        None => {
                            warn!("Skipping `{}` because all id was missing.", &line);
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

                    match records.get(&id) {
                        Some(row) if row.version > version => continue,
                        _ => records.insert(id.to_string(), record),
                    };
                }
                Err(err) => error!("IO error: Line {}: {}", counter, err),
            }
        }
    }

    let mut write_counter = 0;

    for record in records.values() {
        write_counter += 1;
        pb.set_message(&format!(
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

fn find_field<'a>(field: &Vec<String>, json_input: &'a JsonValue) -> Option<&'a JsonValue> {
    let mut value = json_input;

    for part in field {
        value = &value[part];
        if value.is_null() {
            return None;
        }
    }

    return Some(value);
}
