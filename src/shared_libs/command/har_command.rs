use std::fs::File;
use std::io::prelude::*;
use std::io::Error as IoError;
use std::convert::From;
use std::process;
use std::cell::RefCell;

use clap::ArgMatches;
use serde_json;
use serde_json::Error as JsonError;
use url::Url;

pub fn do_har_command(args: &ArgMatches) {
    let input_path = args.value_of("INPUT").unwrap();
    let filtered_domains = args.values_of("filter_domain").map(|x| x.collect()).unwrap_or_else(|| vec![]);
    let filtered_content = args.values_of("filter_context_type").map(|x| x.collect()).unwrap_or_else(|| vec![]);
    let _output = args.values_of("output");

    let json_value = match make_json(input_path) {
        Ok(json_value) => json_value,
        Err(ParseErrors::IO(err)) => {
            error!("Unable to parse {} because {}", input_path, err);
            process::exit(1);
        },
        Err(ParseErrors::Json(err)) => {
            error!("Unable to parse {} because {}", input_path, err);
            process::exit(1);
        }
    };

    let filtered_json = filter_har(json_value, filtered_domains, filtered_content);

    let response_string = serde_json::to_string_pretty(&filtered_json).unwrap();
    println!("{}", response_string);
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HarFile {
    log: LogEntry
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NameVersionEntry {
    name: String, 
    version: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NameValueEntry {
    name: String,
    value: String
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PagesEntry {
    started_date_time: String, 
    id: String,
    title: String,
    page_timings: serde_json::Value
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LogEntry {
    version: String,
    creator: NameVersionEntry,
    browser: NameVersionEntry,
    pages: Vec<PagesEntry>,
    entries: Vec<RequestWrapper>
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct RequestEntry {
    body_size: i32,
    method: String,
    url: String,
    http_version: String,
    headers: Vec<NameValueEntry>,
    cookies: Vec<NameValueEntry>,
    query_string: Vec<NameValueEntry>,
    headers_size: i32
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ResponseEntry {
    status: i32,
    status_text: String,
    http_version: String,
    headers: Vec<NameValueEntry>,
    cookies: serde_json::Value,
    content: serde_json::Value,
    #[serde(rename = "redirectURL")]
    redirect_url: serde_json::Value,
    headers_size: serde_json::Value,
    body_size: serde_json::Value
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct RequestWrapper {
    pageref: String,
    started_date_time: String,
    request: RequestEntry,
    response: ResponseEntry,
    cache: serde_json::Value, 
    timings: serde_json::Value,
    time: serde_json::Value
}

fn filter_har(json_value: HarFile, filtered_domains: Vec<&str>, filtered_content: Vec<&str>) -> HarFile {
    let filtered_domains = RefCell::new(filtered_domains);
    let filtered_content = RefCell::new(filtered_content);

    let filtered_entries = json_value.log.entries.into_iter().filter(|wrapper| {
        let is_domain = is_required_domain(filtered_domains.borrow().to_vec(), wrapper.clone());
        let is_content = is_content_type_correct(filtered_content.borrow().to_vec(), wrapper.clone());

        return is_domain && is_content;
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

fn is_content_type_correct(filtered_content: Vec<&str>, wrapper: RequestWrapper) -> bool{
    if !filtered_content.is_empty() {
        for required_type in filtered_content {
            for header in wrapper.response.headers.clone() {
                if header.name == "Content-Type" {
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