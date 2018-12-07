use std::fs::File;
use std::io::prelude::*;
use std::io::Error as IoError;
use std::convert::From;
use std::cell::{RefCell, Ref};
use std::ops::Deref;

use regex::Regex;
use clap::ArgMatches;
use serde_json;
use serde_json::Error as JsonError;
use url::Url;

use crate::models::har::*;
use crate::output::*;

fn to_regex(input: &str) -> Result<Regex, i32> {
    return match Regex::new(input) {
        Ok(value) => Ok(value),
        Err(err) => {
            error!("Unable to convert {} into a Regex because {}.", input, err);
            return Err(1);
        }
    }
}

pub fn do_har_command(args: &ArgMatches) -> Result<(), i32> {
    let input_path = args.value_of("INPUT").unwrap();
    let filtered_domains = args.values_of("filter_domain").map(|x| x.collect()).unwrap_or_else(|| vec![]);
    let filtered_content = args.values_of("filter_context_type").map(|x| x.collect()).unwrap_or_else(|| vec![]);
    let filter_path = match args.value_of("filter_path") {
        Some(input) => {
            match to_regex(input) {
                Ok(ex) => Some(ex),
                Err(err) => return Err(err)
            }
        },
        None => None
    };

    let writer = match args.value_of("output") {
        Some(path) => Writer::File(FileWriter::new(path.to_string())),
        None => Writer::StdOut(StdOutWriter::new())
    };

    let json_value = match make_json(input_path) {
        Ok(json_value) => json_value,
        Err(ParseErrors::IO(err)) => {
            error!("Unable to parse {} because {}", input_path, err);
            return Err(1);
        },
        Err(ParseErrors::Json(err)) => {
            error!("Unable to parse {} because {}", input_path, err);
            return Err(1);
        }
    };

    let har_file = filter_har(json_value, filtered_domains, filtered_content, filter_path);

    let output_format = match args.value_of("output_format") {
        Some(format) => {
            match format.trim().to_lowercase().as_str() {
                "har" => har_file.to_json(),
                "md" | "markdown" => har_file.to_markdown(),
                "html" => har_file.to_html(),
                _ => {
                    error!("Unable to format to {}", format);
                    return Err(2);
                }        
            }
        }
        _ => unimplemented!()
    };

    return match writer {
        Writer::File(writer) => writer.save(output_format),
        Writer::StdOut(writer) => writer.save(output_format)
    };
}

#[cfg(test)]
mod test {

    use std::path::PathBuf;
    use super::*;

    #[test]
    fn test_doing_filter_without_error() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources");
        d.push("test");
        d.push("example.har");

        println!("Path: {:?}", d);
        make_json(d.to_str().unwrap()).unwrap();
    }
}

fn filter_har(json_value: HarFile, filtered_domains: Vec<&str>, filtered_content: Vec<&str>, filter_path: Option<Regex>) -> HarFile {
    let filtered_domains = RefCell::new(filtered_domains);
    let filtered_content = RefCell::new(filtered_content);
    let filter_path = RefCell::new(filter_path);

    debug!("Wanted domains: {:?}", filtered_domains);
    
    let filtered_entries = json_value.log.entries.into_iter().filter(|wrapper| {
        let is_domain = is_required_domain(filtered_domains.borrow().to_vec(), wrapper.clone());
        let is_content = is_content_type_correct(filtered_content.borrow().to_vec(), wrapper.clone());
        let is_url = is_url_correct(filter_path.borrow(), wrapper.clone());

        debug!("Domain: {}, Content: {}, Url: {}", is_domain,is_content, is_url);

        return is_domain && is_content && is_url;
    }).collect();

    let log = LogEntry { 
        version: json_value.log.version, 
        creator: json_value.log.creator, 
        browser: json_value.log.browser, 
        pages: json_value.log.pages, 
        entries: filtered_entries 
    };

    return HarFile { log: log };
}

fn is_url_correct(filter_path: Ref<Option<Regex>>, wrapper: RequestWrapper) -> bool {
    return match filter_path.deref() {
        None => true,
        Some(ex) => ex.is_match(&wrapper.request.url)
    };
}

fn is_content_type_correct(filtered_content: Vec<&str>, wrapper: RequestWrapper) -> bool{
    if !filtered_content.is_empty() {
        for required_type in filtered_content {
            for header in wrapper.response.headers.clone() {
                if header.name.to_lowercase() == "content-type" {
                    if header.value.contains(required_type) {
                        return true;
                    }
                }
            }
        }
        return false;
    } else {
        return true;
    }
}

fn is_required_domain(filtered_domains: Vec<&str>, wrapper: RequestWrapper) -> bool {
    if !filtered_domains.is_empty() {
        match Url::parse(&wrapper.request.url) {
            Err(err) => error!("Unable to parse URL because {}", err),
            Ok(value) => {
                let domain = value.domain().unwrap();
                if filtered_domains.into_iter().any(|x| domain.contains(x)) {
                    return true;
                }
            }
        }
        
        return false;
    } else {
        return true;
    }
}

#[derive(Debug)]
enum ParseErrors {
    IO(IoError),
    Json(serde_json::Error)
}

impl From<IoError> for ParseErrors {
    fn from(error: IoError) -> Self {
        ParseErrors::IO(error)
    }
}

impl From<JsonError> for ParseErrors {
    fn from(error: JsonError) -> Self {
        ParseErrors::Json(error)
    }
}

fn make_json(input_path: &str) -> Result<HarFile, ParseErrors> {
    let mut file = File::open(input_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    let v: HarFile = serde_json::from_str(&contents)?;
    return Ok(v);
}