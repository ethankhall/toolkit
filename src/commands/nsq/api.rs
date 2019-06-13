use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::prelude::*;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use url::Url;

use super::model::*;
use crate::commands::progress::*;

lazy_static! {
    static ref HTTP_CLIENT: reqwest::r#async::Client = reqwest::r#async::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Should be able to make client");
}

fn do_get(url: &str) -> impl Future<Item = serde_json::Value, Error = String> {
    let url = Url::parse(&url).expect("URL to be valid");

    HTTP_CLIENT
        .get(url)
        .send()
        .map_err(|e| s!(format!("{}", e)))
        .and_then(|resp| {
            let status = resp.status();
            let text = resp
                .into_body()
                .concat2()
                .wait()
                .and_then(|c| {
                    Ok(std::str::from_utf8(&c)
                        .map(str::to_owned)
                        .unwrap_or(s!("no body provided")))
                })
                .unwrap_or(s!("no body provided"));
            if !status.is_success() {
                Err(s!(format!("NSQ returned with an error: {:#?}", text)))
            } else {
                Ok(s!(text))
            }
        })
        .and_then(|json_body| match serde_json::from_str(&json_body) {
            Ok(value) => Ok(value),
            Err(e) => Err(s!(format!("JSON Deseralization error: {:?}", e))),
        })
}

#[derive(Debug)]
enum ErrorType {
    Fatal(String),
}

#[derive(Debug, Clone, Serialize)]
pub enum NsqFilter {
    Producer {
        hosts: BTreeSet<String>,
    },
    Topic {
        topics: BTreeSet<String>,
    },
    ProducerAndTopic {
        hosts: BTreeSet<String>,
        topics: BTreeSet<String>,
    },
}

#[derive(Debug, Serialize)]
pub struct NsqState {
    host_details: BTreeMap<String, HostDetails>,
}

#[derive(Debug, Serialize)]
struct HostDetails {
    hostname: String,
    base_url: String,
    topics: BTreeSet<String>,
}

#[derive(Debug, Serialize)]
struct HostStatus {
    hostname: String,
    status: Vec<HostTopicStatus>,
}

impl NsqState {
    pub fn new(nsq_lookup: &str, filter: NsqFilter) -> Self {
        let mut runtime = Runtime::new().unwrap();

        let host_details_fut =
            do_get(&format!("http://{}/nodes", nsq_lookup)).and_then(|nodes_response| {
                let mut host_details: BTreeMap<String, HostDetails> = BTreeMap::new();
                let empty_vec: Vec<serde_json::Value> = vec![];
                let producers = nodes_response["data"]["producers"]
                    .as_array()
                    .unwrap_or(&empty_vec);

                for producer in producers {
                    let hostname = producer["hostname"].as_str().unwrap();
                    let port = producer["http_port"].as_u64().unwrap();
                    let topics = producer["topics"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| s!(x.as_str().unwrap()))
                        .collect();
                    host_details
                        .entry(s!(hostname))
                        .or_insert_with(|| HostDetails::new(hostname, port, topics));
                }

                Ok(host_details)
            });

        let mut host_details = runtime
            .block_on(host_details_fut)
            .expect("To be able to talk to NSQ");

        let (topics_to_include, producers_to_include) = match filter {
            NsqFilter::Producer { hosts } => (None, Some(hosts)),
            NsqFilter::ProducerAndTopic { hosts, topics } => (Some(topics), Some(hosts)),
            NsqFilter::Topic { topics } => (Some(topics), None),
        };

        if let Some(producers) = producers_to_include {
            let mut remove_list = vec![];

            for (key, _host) in host_details.iter() {
                if producers.contains(key) {
                    remove_list.push(key.clone());
                }
            }

            for host in remove_list {
                host_details.remove(&host);
            }
        }

        if let Some(topics) = topics_to_include {
            let mut remove_list = vec![];

            for (key, host) in host_details.iter_mut() {
                let mut intersection: BTreeSet<_> = host
                    .topics
                    .intersection(&topics)
                    .map(|x| x.clone())
                    .collect();
                host.topics.clear();
                host.topics.append(&mut intersection);

                if host.topics.len() == 0 {
                    remove_list.push(key.clone());
                }
            }

            for host in remove_list {
                host_details.remove(&host);
            }
        }

        NsqState { host_details }
    }

    pub fn update_status<'a>(&self) -> NsqSnapshot {
        let pb = ProgressBarHelper::new(ProgressBarType::SizedProgressBar(self.host_details.len(), "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} Fetching status from NSQ Hosts"));
        let pb = Arc::new(pb);

        let mut runtime = Runtime::new().unwrap();
        let mut futures: Vec<_> = Vec::new();
        for (_key, value) in self.host_details.iter() {
            let future = value.create_host_status();
            let pb = pb.clone();
            futures.push(future.and_then(move |x| {
                pb.inc();
                Ok(x)
            }));
        }

        let uber_future = future::join_all(futures);
        let statuses = runtime
            .block_on(uber_future)
            .expect("API to be somewhat stable");

        self.make_snapshot(statuses)
    }

    fn make_snapshot(&self, host_status_vec: Vec<HostStatus>) -> NsqSnapshot {
        let mut snapshot = NsqSnapshot {
            pull_finished: Local::now(),
            topics: BTreeMap::new(),
            producers: BTreeMap::new(),
        };

        for details in host_status_vec {
            for topic_status in details.status.iter() {
                let topic_name = topic_status.topic_name.clone();
                let producer_hostname = details.hostname.clone();

                let host_details = self.host_details.get(&producer_hostname).unwrap();

                if host_details.topics.contains(&topic_name) {
                    let topic_snapshot = snapshot
                        .topics
                        .entry(topic_name.clone())
                        .or_insert_with(|| NsqTopicSnapshot::new(topic_name));
                    let producer_snapshot = NsqTopicProducerSnapshot::new(
                        producer_hostname.clone(),
                        topic_status.message_count,
                        topic_status.depth,
                    );
                    topic_snapshot
                        .producers
                        .insert(producer_hostname.clone(), producer_snapshot);
                    for channel in topic_status.channels.iter() {
                        let channel_name = channel.channel_name.clone();
                        let channel_snapshot = topic_snapshot
                            .consumers
                            .entry(channel_name.clone())
                            .or_insert_with(|| NsqTopicConsumerSnapshot::new(channel_name));
                        channel_snapshot.merge(channel);
                    }
                }

                for channel in topic_status.channels.iter() {
                    let producer_agg = snapshot
                        .producers
                        .entry(producer_hostname.clone())
                        .or_insert(NsqTopicProducerAggregate {
                            hostname: producer_hostname.clone(),
                            depth: 0,
                            message_count: 0,
                        });
                    producer_agg.merge(channel);
                }
            }
        }

        snapshot
    }

    pub fn get_topic_url(&self, topic_name: &str) -> Option<String> {
        self.host_details
            .values()
            .find(|host| host.topics.iter().any(|x| x == topic_name))
            .map(|x| format!("http://{}", x.hostname))
    }
}

pub struct NsqSnapshot {
    pub pull_finished: DateTime<Local>,
    pub topics: BTreeMap<String, NsqTopicSnapshot>,
    pub producers: BTreeMap<String, NsqTopicProducerAggregate>,
}

impl NsqSnapshot {
    pub fn get_channel(
        &self,
        topic: &str,
        channel_name: &str,
    ) -> Option<&NsqTopicConsumerSnapshot> {
        self.topics
            .get(topic)
            .and_then(|topic_snapshot| topic_snapshot.consumers.get(channel_name))
    }
}

pub struct NsqTopicSnapshot {
    pub name: String,
    pub consumers: BTreeMap<String, NsqTopicConsumerSnapshot>,
    pub producers: BTreeMap<String, NsqTopicProducerSnapshot>,
}

pub struct NsqTopicProducerAggregate {
    pub hostname: String,
    pub depth: u64,
    pub message_count: u64,
}

impl NsqTopicProducerAggregate {
    fn merge(&mut self, status: &ChannelStatus) {
        self.depth += status.depth;
        self.message_count += status.message_count
    }
}

impl NsqTopicSnapshot {
    fn new(name: String) -> Self {
        NsqTopicSnapshot {
            name,
            consumers: BTreeMap::new(),
            producers: BTreeMap::new(),
        }
    }

    pub fn producer_aggregate(&self) -> NsqTopicProducerAggregate {
        let mut aggregate = NsqTopicProducerAggregate {
            hostname: s!(""),
            depth: 0,
            message_count: 0,
        };

        for producer in self.producers.values() {
            aggregate.depth += producer.depth;
            aggregate.message_count += producer.message_count;
        }

        aggregate
    }
}

pub struct NsqTopicConsumerSnapshot {
    pub channel_name: String,
    pub finish_count: u64,
    pub in_progress: u64,
    pub depth: u64,
}

impl NsqTopicConsumerSnapshot {
    fn new(channel_name: String) -> Self {
        NsqTopicConsumerSnapshot {
            channel_name,
            finish_count: 0,
            in_progress: 0,
            depth: 0,
        }
    }

    fn merge(&mut self, channel_status: &ChannelStatus) {
        self.depth += channel_status.depth;
        self.in_progress += channel_status.in_flight_count;
        self.finish_count += channel_status.message_count;
    }
}

pub struct NsqTopicProducerSnapshot {
    pub hostname: String,
    pub message_count: u64,
    pub depth: u64,
}

impl NsqTopicProducerSnapshot {
    fn new(hostname: String, message_count: u64, depth: u64) -> Self {
        NsqTopicProducerSnapshot {
            hostname,
            message_count,
            depth,
        }
    }
}

impl HostDetails {
    fn new(hostname: &str, port: u64, topic: Vec<String>) -> Self {
        use std::iter::FromIterator;

        HostDetails {
            hostname: s!(hostname),
            base_url: format!("{}:{}", hostname, port),
            topics: BTreeSet::from_iter(topic.into_iter()),
        }
    }

    fn create_host_status(&self) -> impl Future<Item = HostStatus, Error = ErrorType> {
        let hostname = self.hostname.clone();

        do_get(&format!("http://{}/stats?format=json", self.base_url))
            .map_err(|err| ErrorType::Fatal(format!("{}", err)))
            .and_then(
                |root| match serde_json::from_value::<StatusTopicsDetails>(root.clone()) {
                    Ok(details) => Ok(Some(details)),
                    Err(_) => match serde_json::from_value::<StatusTopicsResponse>(root.clone()) {
                        Ok(root) => Ok(Some(root.data)),
                        Err(err) => {
                            warn!(
                                "Unable to deserialize {} from the stats because {:?}",
                                root, err
                            );
                            return Ok(None);
                        }
                    },
                },
            )
            .and_then(|json_obj| {
                let mut result: Vec<HostTopicStatus> = Vec::new();

                let json_obj = match json_obj {
                    Some(obj) => obj,
                    None => {
                        return Ok(HostStatus {
                            hostname: hostname,
                            status: result,
                        })
                    }
                };

                for topic in json_obj.topics {
                    let topic_name = topic.topic_name;
                    let depth = topic.depth;
                    let message_count = topic.message_count;

                    let channels: Vec<ChannelStatus> = topic
                        .channels
                        .into_iter()
                        .map(|channel| ChannelStatus::new(topic_name.clone(), channel))
                        .collect();

                    let topic_status = HostTopicStatus {
                        topic_name: topic_name.clone(),
                        depth,
                        message_count,
                        channels,
                    };

                    result.push(topic_status);
                }

                Ok(HostStatus {
                    hostname: hostname,
                    status: result,
                })
            })
    }
}

#[derive(Debug, Serialize)]
struct HostTopicStatus {
    pub topic_name: String,
    pub depth: u64,
    pub message_count: u64,
    pub channels: Vec<ChannelStatus>,
}

#[derive(Debug, Serialize)]
struct ChannelStatus {
    pub channel_name: String,
    pub topic_name: String,
    pub depth: u64,
    pub in_flight_count: u64,
    pub message_count: u64,
    pub consumers: Vec<ConsumerHost>,
}

impl ChannelStatus {
    fn new(topic_name: String, channel: TopicChannel) -> Self {
        let consumers: Vec<ConsumerHost> = channel
            .clients
            .iter()
            .map(|x| ConsumerHost::new(x))
            .collect();
        ChannelStatus {
            channel_name: channel.channel_name,
            topic_name,
            depth: channel.depth,
            in_flight_count: channel.in_flight_count,
            message_count: channel.message_count,
            consumers,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
struct ConsumerHost {
    hostname: String,
}

impl ConsumerHost {
    fn new(details: &ClientDetails) -> Self {
        ConsumerHost {
            hostname: details.hostname.clone(),
        }
    }
}
