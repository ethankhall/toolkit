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
    pub message_count: u64,
    pub clients: Vec<ClientDetails>
}

#[derive(Serialize, Deserialize)]
pub struct ClientDetails {
    pub hostname: String,
}

#[derive(Serialize, Deserialize)]
pub struct LookupProducer {
    pub remote_address: String,
    pub hostname: String,
    pub broadcast_address: String,
    pub tcp_port: i32,
    pub http_port: i32,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct LookupResponse {
    pub status_code: i32,
    pub data: LookupData,
}

#[derive(Serialize, Deserialize)]
pub struct LookupData {
    pub producers: Vec<LookupProducer>,
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

    serde_json::from_str::<StatusTopicsDetails>(body).unwrap();
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

    serde_json::from_str::<StatusTopicsResponse>(body).unwrap();
}
