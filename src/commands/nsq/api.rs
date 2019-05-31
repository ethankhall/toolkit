use std::sync::Mutex;
use std::collections::{HashSet, BTreeMap};
use std::collections::BTreeSet;

use crate::commands::progress::*;
use super::TopicUrlElement;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct StatusTopicsResponse {
    pub data: StatusTopicsDetails,
}

#[derive(Serialize, Deserialize)]
pub struct StatusTopicsDetails {
    pub topics: Vec<TopicDetails>,
}

#[derive(Serialize, Deserialize)]
pub struct TopicDetails {
    pub topic_name: String,
    pub depth: u64,
    pub message_count: u64,
    pub channels: Vec<TopicChannel>,
}

#[derive(Serialize, Deserialize)]
pub struct TopicChannel {
    pub depth: u64,
    pub in_flight_count: u64,
    pub channel_name: String,
    pub clients: Vec<ClientDetails>
}

#[derive(Serialize, Deserialize)]
pub struct ClientDetails {
    pub hostname: String
}

#[derive(Serialize, Deserialize)]
struct LookupProducer {
    remote_address: String,
    hostname: String,
    broadcast_address: String,
    tcp_port: i32,
    http_port: i32,
    version: String,
}

#[derive(Serialize, Deserialize)]
struct LookupResponse {
    status_code: i32,
    data: LookupData,
}

#[derive(Serialize, Deserialize)]
struct LookupData {
    producers: Vec<LookupProducer>,
}

lazy_static! {
    static ref PING_VALIDATION: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

pub fn get_base_url_for_topic(nsq_lookup: &str, topic: &str) -> Option<TopicUrlElement> {
    let url = format!("http://{}/lookup?topic={}", nsq_lookup, topic);
    let url = Url::parse(&url).expect("URL to be valid");
    let body = match reqwest::get(url) {
        Err(e) => {
            error!("Unable to talk to NSQ: {}", e.to_string());
            return None;
        }
        Ok(mut resp) => {
            if !resp.status().is_success() {
                error!("NSQ returned with an error: {:#?}", resp.text());
                return None;
            } else {
                resp.text().unwrap()
            }
        }
    };

    let json_body: LookupResponse =
        serde_json::from_str(&body).expect("To be able to get LookupResponse from NSQ API");
    let mut hosts: BTreeSet<String> = BTreeSet::new();

    for producer in json_body.data.producers {
        let base_url = format!(
            "http://{}:{}",
            producer.broadcast_address, producer.http_port
        );

        let mut lock = PING_VALIDATION.lock().unwrap();

        if !lock.contains(&base_url) {
            let url = format!("{}/ping", base_url.clone());
            let url = Url::parse(&url).expect("URL to be valid");
            if let Ok(_) = reqwest::get(url) {
                lock.insert(base_url.clone());
            }
        }

        if lock.contains(&base_url) {
            hosts.insert(base_url.clone());
        }
    }

    if hosts.is_empty() {
        error!("Unable to connect to NSQ host to send messages!");
        return None;
    }

    return Some(TopicUrlElement::new(s!(topic), hosts));
}

pub fn get_base_url_for_hosts(nsq_lookup: &str, hosts: &[&str]) -> Option<Vec<TopicUrlElement>> {
    let mut host_set: HashSet<String> = HashSet::new();
    for host in hosts {
        host_set.insert(s!(host));
    }

    let url = format!("http://{}/nodes", nsq_lookup);
    let url = Url::parse(&url).expect("URL to be valid");
    let body = match reqwest::get(url) {
        Err(e) => {
            error!("Unable to talk to NSQ: {}", e.to_string());
            return None;
        }
        Ok(mut resp) => {
            if !resp.status().is_success() {
                error!("NSQ returned with an error: {:#?}", resp.text());
                return None;
            } else {
                resp.text().unwrap()
            }
        }
    };

    let root_json: serde_json::Value = serde_json::from_str(&body).expect("To be able to get LookupResponse from NSQ API");
    let empty_vec: Vec<serde_json::Value> = vec![];
    let producers = root_json["data"]["producers"].as_array().unwrap_or(&empty_vec);

    let mut topic_map: BTreeMap<String, TopicUrlElement> = BTreeMap::new();
    let mut topic_set: HashSet<String> = HashSet::new();

    let pb = ProgressBarHelper::new(ProgressBarType::SizedProgressBar(producers.len(), "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}"));

    for producer in producers {
        let hostname = producer["hostname"].as_str().unwrap();
        pb.inc_with_message(&format!("Checking on {}", hostname));
        let url = format!("http://{}:{}/stats?format=json", hostname, producer["http_port"].as_u64().unwrap());
        if let Some(data) = get_topic_status_from_url(&url) {
            data.topics.into_iter().filter(|topic| {
                return topic.channels.iter().any(|channel| {
                    return channel.clients.iter().any(|x| host_set.contains(&x.hostname));
                });
            })
            .for_each(|topic| {
                topic_set.insert(topic.topic_name);
            });
        }
    }
    
    for topic in topic_set {
        if let Some(found_topic) = get_base_url_for_topic(nsq_lookup, &topic) {
            let topic_name = format!("{}", topic);
            if !topic_map.contains_key(&topic_name.clone()) {
                topic_map.insert(topic_name.clone(), TopicUrlElement::new(topic_name.clone(), BTreeSet::new()));
            }

            topic_map.get_mut(&topic_name).unwrap().add_urls(found_topic.urls);
        }
    }

    let result: Vec<TopicUrlElement> = topic_map.into_iter().map(|(_, v)| v).collect();
    Some(result)
}

pub fn get_queue_size(base_url: &str, topic: &str) -> Option<(u64, u64)> {
    match get_topic_status(base_url, topic) {
        Some(root) => extract_size_from_body(root, topic),
        None => None,
    }
}

pub fn get_topic_status(base_url: &str, topic: &str) -> Option<StatusTopicsDetails> {
    let topic_url = format!("{}/stats?format=json&topic={}", base_url, topic);
    get_topic_status_from_url(&topic_url) 
}

fn get_topic_status_from_url(url: &str) -> Option<StatusTopicsDetails> {
    let topic_url = Url::parse(&url).expect("URL to be valid");

    if let Ok(mut response) = reqwest::get(topic_url) {
        if let Ok(body) = response.text() {
            let json_body: Result<StatusTopicsDetails, _> = serde_json::from_str(&body);
            if let Ok(root) = json_body {
                return Some(root);
            } else {
                let json_body: Result<StatusTopicsResponse, serde_json::error::Error> = serde_json::from_str(&body);
                return match json_body {
                    Ok(root) => Some(root.data),
                    Err(err) => {
                        warn!("Unable to deserialize {} from the stats because {:?}", body, err);
                        return None;
                    }
                }
            }
        }
    }
    None
}

fn extract_size_from_body(body: StatusTopicsDetails, topic: &str) -> Option<(u64, u64)> {
    let topic_details: Option<TopicDetails> =
        body.topics.into_iter().find(|x| x.topic_name == topic);

    match topic_details {
        None => None,
        Some(topic) => {
            let mut queued = topic.depth;
            let mut in_flight: u64 = 0;
            for channel in topic.channels {
                if channel.depth > queued {
                    queued = channel.depth;
                }

                if channel.in_flight_count > in_flight {
                    in_flight = channel.in_flight_count;
                }
            }
            Some((queued, in_flight))
        }
    }
}

#[test]
fn test_extract_size() {
    let body = "{
    \"version\": \"1.1.0\",
    \"health\": \"OK\",
    \"start_time\": 1548185315,
    \"topics\": [
        {
            \"topic_name\": \"foo\",
            \"channels\": [
                {
                    \"channel_name\": \"tail180292#ephemeral\",
                    \"depth\": 3,
                    \"backend_depth\": 0,
                    \"in_flight_count\": 1,
                    \"deferred_count\": 0,
                    \"message_count\": 1399,
                    \"requeue_count\": 0,
                    \"timeout_count\": 0,
                    \"clients\": [
                        {
                            \"client_id\": \"ethan\",
                            \"hostname\": \"ethan.local\",
                            \"version\": \"V2\",
                            \"remote_address\": \"1.2.3.4:33576\",
                            \"state\": 3,
                            \"ready_count\": 1,
                            \"in_flight_count\": 1,
                            \"message_count\": 1396,
                            \"finish_count\": 1395,
                            \"requeue_count\": 0,
                            \"connect_ts\": 1549065745,
                            \"sample_rate\": 0,
                            \"deflate\": false,
                            \"snappy\": false,
                            \"user_agent\": \"nsq_tail/1.1.0 go-nsq/1.0.6\",
                            \"tls\": false,
                            \"tls_cipher_suite\": \"\",
                            \"tls_version\": \"\",
                            \"tls_negotiated_protocol\": \"\",
                            \"tls_negotiated_protocol_is_mutual\": false
                        }
                    ],
                    \"paused\": false,
                    \"e2e_processing_latency\": {
                        \"count\": 0,
                        \"percentiles\": null
                    }
                }
            ],
            \"depth\": 0,
            \"backend_depth\": 0,
            \"message_count\": 29259,
            \"paused\": false,
            \"e2e_processing_latency\": {
                \"count\": 0,
                \"percentiles\": null
            }
        }
    ],
    \"memory\": {
        \"heap_objects\": 21625,
        \"heap_idle_bytes\": 11886592,
        \"heap_in_use_bytes\": 3743744,
        \"heap_released_bytes\": 10280960,
        \"gc_pause_usec_100\": 5612,
        \"gc_pause_usec_99\": 3742,
        \"gc_pause_usec_95\": 878,
        \"next_gc_bytes\": 4194304,
        \"gc_total_runs\": 219
    }
}";

    let body: StatusTopicsDetails = serde_json::from_str(body).unwrap();

    let (queued, in_flight) = extract_size_from_body(body, "foo").unwrap();
    assert_eq!(3, queued);
    assert_eq!(1, in_flight);
}

#[test]
fn older_api_test() {
    let body = "{
  \"status_code\": 200,
  \"status_txt\": \"OK\",
  \"data\": {
    \"version\": \"0.3.8\",
    \"health\": \"OK\",
    \"start_time\": 1543350728,
    \"topics\": [
        {
            \"topic_name\": \"foo\",
            \"channels\": [
                {
                    \"channel_name\": \"tail180292#ephemeral\",
                    \"depth\": 3,
                    \"backend_depth\": 0,
                    \"in_flight_count\": 1,
                    \"deferred_count\": 0,
                    \"message_count\": 1399,
                    \"requeue_count\": 0,
                    \"timeout_count\": 0,
                    \"clients\": [
                        {
                            \"client_id\": \"ethan\",
                            \"hostname\": \"ethan.local\",
                            \"version\": \"V2\",
                            \"remote_address\": \"1.2.3.4:33576\",
                            \"state\": 3,
                            \"ready_count\": 1,
                            \"in_flight_count\": 1,
                            \"message_count\": 1396,
                            \"finish_count\": 1395,
                            \"requeue_count\": 0,
                            \"connect_ts\": 1549065745,
                            \"sample_rate\": 0,
                            \"deflate\": false,
                            \"snappy\": false,
                            \"user_agent\": \"nsq_tail/1.1.0 go-nsq/1.0.6\",
                            \"tls\": false,
                            \"tls_cipher_suite\": \"\",
                            \"tls_version\": \"\",
                            \"tls_negotiated_protocol\": \"\",
                            \"tls_negotiated_protocol_is_mutual\": false
                        }
                    ],
                    \"paused\": false,
                    \"e2e_processing_latency\": {
                        \"count\": 0,
                        \"percentiles\": null
                    }
                }
            ],
            \"depth\": 0,
            \"backend_depth\": 0,
            \"message_count\": 29259,
            \"paused\": false,
            \"e2e_processing_latency\": {
                \"count\": 0,
                \"percentiles\": null
            }
        }
    ],
    \"memory\": {
        \"heap_objects\": 21625,
        \"heap_idle_bytes\": 11886592,
        \"heap_in_use_bytes\": 3743744,
        \"heap_released_bytes\": 10280960,
        \"gc_pause_usec_100\": 5612,
        \"gc_pause_usec_99\": 3742,
        \"gc_pause_usec_95\": 878,
        \"next_gc_bytes\": 4194304,
        \"gc_total_runs\": 219
        }
    }
}";

    let body: StatusTopicsResponse = serde_json::from_str(body).unwrap();

    let (queued, in_flight) = extract_size_from_body(body.data, "foo").unwrap();
    assert_eq!(3, queued);
    assert_eq!(1, in_flight);
}
