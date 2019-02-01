use url::Url;

#[derive(Serialize, Deserialize)]
struct StatusDetails {
    data: StatusTopicsDetails
}

#[derive(Serialize, Deserialize)]
struct StatusTopicsDetails {
    topics: Vec<TopicDetails>
}

#[derive(Serialize, Deserialize)]
struct TopicDetails {
    topic_name: String,
    channels: Vec<TopicChannel>,
}

#[derive(Serialize, Deserialize)]
struct TopicChannel {
    depth: i32,
    in_flight_count: i32,
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

pub fn get_base_url_for_topic(nsq_lookup: String, topic: &str) -> Option<String> {
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
    let topic_url = format!("{}/stats?format=json&topic={}", base_url, topic);
    let topic_url = Url::parse(&topic_url).expect("URL to be valid");

    if let Ok(mut response) = reqwest::get(topic_url) {
        if let Ok(body) = response.text() {
            let json_body: Result<StatusDetails, _> = serde_json::from_str(&body);
            if let Ok(root) = json_body {
                let channels: Vec<TopicChannel> = root.data.topics.into_iter().flat_map(|x|{
                    if x.topic_name != topic {
                        return vec!();
                    } else {
                        return x.channels;
                    }
                }).collect();
                let mut queued = 0;
                let mut in_flight = 0;
                for channel in channels {
                    if channel.depth > queued {
                        queued = channel.depth;
                    }

                    if channel.in_flight_count > in_flight {
                        in_flight = channel.in_flight_count;
                    }
                }
                return Some((queued, in_flight));
            }
        }
    }
    None
}