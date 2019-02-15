use url::Url;

#[derive(Serialize, Deserialize)]
pub struct StatusTopicsDetails {
    pub topics: Vec<TopicDetails>
}

#[derive(Serialize, Deserialize)]
pub struct TopicDetails {
    pub topic_name: String,
    pub depth: i32,
    pub channels: Vec<TopicChannel>,
}

#[derive(Serialize, Deserialize)]
pub struct TopicChannel {
    pub depth: i32,
    pub in_flight_count: i32,
    pub channel_name: String,
}

#[derive(Serialize, Deserialize)]
struct LookupProducer {
    remote_address: String,
    hostname: String,
    broadcast_address: String,
    tcp_port: i32,
    http_port: i32,
    version: String
}

#[derive(Serialize, Deserialize)]
struct LookupResponse {
    status_code: i32,
    data: LookupData
}

#[derive(Serialize, Deserialize)]
struct LookupData {
    producers: Vec<LookupProducer>
}

pub fn get_base_url_for_topic(nsq_lookup: &str, topic: &str) -> Option<String> {
    let url = format!("http://{}/lookup?topic={}", nsq_lookup, topic);
    let url = Url::parse(&url).expect("URL to be valid");
    let body = match reqwest::get(url) {
        Err(e) => { 
            error!("Unable to talk to NSQ: {}", e.to_string());
            return None;
        },
        Ok(mut resp) => {
            if !resp.status().is_success() {
                error!("NSQ returned with an error: {:#?}", resp.text());
                return None;
            } else {
                resp.text().unwrap()
            }
        }
    };

    let json_body: LookupResponse = serde_json::from_str(&body).expect("To be able to get LookupResponse from NSQ API");
    for producer in json_body.data.producers {
        let base_url = format!("http://{}:{}", producer.broadcast_address, producer.http_port);

        let url = format!("{}/ping", base_url);
        let url = Url::parse(&url).expect("URL to be valid");
        if let Ok(_) = reqwest::get(url) {
            return Some(base_url);
        }
    }

    error!("Unable to connect to NSQ host to send messages!");
    None
}

pub fn get_queue_size(base_url: &str, topic: &str) -> Option<(i32, i32)> {
    match get_topic_status(base_url, topic) {
        Some(root) => extract_size_from_body(root, topic),
        None => None
    }
}

pub fn get_topic_status(base_url: &str, topic: &str) -> Option<StatusTopicsDetails> {
    let topic_url = format!("{}/stats?format=json&topic={}", base_url, topic);
    let topic_url = Url::parse(&topic_url).expect("URL to be valid");

    if let Ok(mut response) = reqwest::get(topic_url) {
        if let Ok(body) = response.text() {
            let json_body: Result<StatusTopicsDetails, _> = serde_json::from_str(&body);
            if let Ok(root) = json_body {
                return Some(root);
            } else {
                warn!("Unable to deserialize {} from the stats", body);
            }
        }
    }
    None
}

fn extract_size_from_body(body: StatusTopicsDetails, topic: &str) -> Option<(i32, i32)> {
    let topic_details: Option<TopicDetails> = body.topics.into_iter().find(|x| x.topic_name == topic);

    match topic_details {
        None => None,
        Some(topic) => {
            let mut queued = topic.depth;
            let mut in_flight = 0;
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