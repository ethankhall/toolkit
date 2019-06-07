use std::sync::RwLock;
use std::collections::{BTreeMap, BTreeSet};

use futures::future::{join_all};
use chrono::prelude::*;
use url::Url;

use crate::commands::progress::*;
use super::model::*;

lazy_static! {
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::builder().build().expect("Should be able to make client");
}

fn do_get(url: &str) -> Option<serde_json::Value> {
    let url = Url::parse(&url).expect("URL to be valid");

    let body = match HTTP_CLIENT.get(url).send() {
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

    match serde_json::from_str(&body) {
        Ok(value) => Some(value),
        Err(e) => {
            error!("JSON Deseralization error: {:?}", e);
            None
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum NsqFilter {
    Host { hosts: BTreeSet<String> },
    Topic { topics: BTreeSet<String> },
    HostAndTopic { hosts: BTreeSet<String>, topics: BTreeSet<String> }
}

impl NsqFilter {
    fn include_topic(&self, topic_name: &str) -> bool {
        match self {
            NsqFilter::Host { hosts: _ } => true,
            NsqFilter::HostAndTopic { hosts: _, topics } => topics.iter().any(|topic| topic == topic_name),
            NsqFilter::Topic { topics } => topics.iter().any(|topic| topic == topic_name),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NsqState {
    host_details: BTreeMap<String, HostDetails>,
    filter: NsqFilter
}

#[derive(Debug, Serialize)]
struct HostDetails {
    hostname: String,
    status: RwLock<Vec<HostTopicStatus>>,
    disable_host: bool
}


impl NsqState {
    pub async fn new(nsq_lookup: &str, filter: NsqFilter) -> Self {
        let mut host_details: BTreeMap<String, HostDetails> = BTreeMap::new();

        let nodes_response = do_get(&format!("http://{}/nodes", nsq_lookup)).expect("Unable to talk to nsq_lookup");
        let empty_vec: Vec<serde_json::Value> = vec![];
        let producers = nodes_response["data"]["producers"].as_array().unwrap_or(&empty_vec);

        for producer in producers {
            let hostname = producer["hostname"].as_str().unwrap();
            let port = producer["http_port"].as_u64().unwrap();
            host_details.entry(s!(hostname)).or_insert_with(|| HostDetails::new(format!("{}:{}", hostname, port)));
        }

        let pb = ProgressBarHelper::new(ProgressBarType::SizedProgressBar(host_details.len(), "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} Waiting for NSQ nodes to respond"));

        let futures: Vec<_> = host_details.values_mut().map(|x| x.update_status(&pb)).collect();
        join_all(futures).await;

        host_details.values_mut().for_each(|x| x.initalize(&filter));

        pb.done();

        NsqState {
            host_details,
            filter
        }
    }

    pub async fn update_status(&self) -> NsqSnapshot {

        let pb = ProgressBarHelper::new(ProgressBarType::SizedProgressBar(self.host_details.len(), "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} Fetching status from NSQ Hosts"));

        let mut futures = Vec::new();
        for value in self.host_details.values() {
            futures.push(value.update_status(&pb));
        }

        join_all(futures).await;

        self.get_status()
    }

    pub fn get_status(&self) -> NsqSnapshot {
        let mut snapshot = NsqSnapshot { pull_finished: Local::now(), topics: BTreeMap::new(), producers: BTreeMap::new() };

        for details in self.host_details.values().filter(|x| x.disable_host == false) {
            for topic_status in details.status.read().unwrap().iter() {
                let topic_name = topic_status.topic_name.clone();
                let producer_hostname = details.hostname.clone();

                if self.filter.include_topic(&topic_name) {
                    let topic_snapshot = snapshot.topics.entry(topic_name.clone()).or_insert_with(|| NsqTopicSnapshot::new(topic_name));
                    let producer_snapshot = NsqTopicProducerSnapshot::new(producer_hostname.clone(), topic_status.message_count, topic_status.depth);
                    topic_snapshot.producers.insert(producer_hostname.clone(), producer_snapshot);
                    for channel in topic_status.channels.iter() {
                        let channel_name = channel.channel_name.clone();
                        let channel_snapshot = topic_snapshot.consumers.entry(channel_name.clone()).or_insert_with(|| NsqTopicConsumerSnapshot::new(channel_name));
                        channel_snapshot.merge(channel);
                    }
                }

                for channel in topic_status.channels.iter() {
                    let producer_agg = snapshot.producers.entry(producer_hostname.clone()).or_insert(NsqTopicProducerAggregate { hostname: producer_hostname.clone(), depth: 0, message_count: 0});
                    producer_agg.merge(channel);
                }
            }
        }

        snapshot
    }

    pub fn get_topic_url(&self, topic_name: &str) -> Option<String> {
        self.host_details.values()
            .find(|host| host.status.read().unwrap().iter().any(|x| x.topic_name == topic_name))
            .map(|x| format!("http://{}", x.hostname))
    }
}

pub struct NsqSnapshot {
    pub pull_finished: DateTime<Local>,
    pub topics: BTreeMap<String, NsqTopicSnapshot>,
    pub producers: BTreeMap<String, NsqTopicProducerAggregate>,
}

impl NsqSnapshot {
    pub fn get_channel(&self, topic: &str, channel_name: &str) -> Option<&NsqTopicConsumerSnapshot> {
        self.topics.get(topic).and_then(|topic_snapshot| topic_snapshot.consumers.get(channel_name))
    }
}

pub struct NsqTopicSnapshot {
    pub name: String, 
    pub consumers: BTreeMap<String, NsqTopicConsumerSnapshot>,
    pub producers: BTreeMap<String, NsqTopicProducerSnapshot>
}

pub struct NsqTopicProducerAggregate {
    pub hostname: String,
    pub depth: u64,
    pub message_count: u64
}

impl NsqTopicProducerAggregate {
    fn merge(&mut self, status: &ChannelStatus) {
        self.depth += status.depth;
        self.message_count += status.message_count
    }
}

impl NsqTopicSnapshot {
    fn new(name: String) -> Self {
        NsqTopicSnapshot { name, consumers: BTreeMap::new(), producers: BTreeMap::new() }
    }

    pub fn producer_aggregate(&self) -> NsqTopicProducerAggregate {
        let mut aggregate = NsqTopicProducerAggregate { hostname: s!(""), depth: 0, message_count: 0 };

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
    pub depth: u64
}

impl NsqTopicConsumerSnapshot {
    fn new(channel_name: String) -> Self {
        NsqTopicConsumerSnapshot { channel_name, finish_count: 0, in_progress: 0, depth: 0}
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
    pub depth: u64
}

impl NsqTopicProducerSnapshot {
    fn new(hostname: String, message_count: u64, depth: u64) -> Self {
        NsqTopicProducerSnapshot { hostname, message_count, depth }
    }
}

impl HostDetails {
    fn new(hostname: String) -> Self {
        HostDetails {
            hostname: hostname,
            status: RwLock::new(Vec::new()),
            disable_host: false
        }
    }

    fn initalize<'a>(&'a mut self, filter: &'a NsqFilter) {
        let consumer_hostnames: BTreeSet<String> = self.status.read().unwrap().iter()
            .flat_map(|status| status.channels.iter())
            .flat_map(|channel| channel.consumers.iter().map(|consumer| consumer.hostname.clone()))
            .collect();

        let topic_names: BTreeSet<String> = self.status.read().unwrap().iter()
            .map(|status| status.topic_name.clone())
            .collect();

        match filter {
            NsqFilter::Host { hosts } => {
                self.disable_host = hosts.intersection(&consumer_hostnames).count() == 0
            },
            NsqFilter::HostAndTopic { hosts, topics } => {
                if hosts.intersection(&consumer_hostnames).count() == 0 {
                    self.disable_host = true;
                } else {
                    self.disable_host = topics.intersection(&topic_names).count() == 0;
                }
            }
            NsqFilter::Topic { topics } => {
                self.disable_host = topics.intersection(&topic_names).count() == 0;
            }
        }
    }

    async fn update_status<'a>(&'a self, pb: &'a ProgressBarHelper) {
        if self.disable_host {
            pb.inc();
            return;
        }

        let root = match do_get(&format!("http://{}/stats?format=json", self.hostname)) {
            Some(root) => root,
            None => {
                return;
            }
        };

        let details = match serde_json::from_value::<StatusTopicsDetails>(root.clone()) {
            Ok(details) => details,
            Err(_) => {
                match serde_json::from_value::<StatusTopicsResponse>(root.clone()) {
                    Ok(root) => root.data,
                    Err(err) => {
                        warn!("Unable to deserialize {} from the stats because {:?}", root, err);
                        return;
                    }
                }
            }
        };

        let mut result: Vec<HostTopicStatus> = Vec::new();

        for topic in details.topics {
            let topic_name = topic.topic_name;
            let depth = topic.depth;
            let message_count = topic.message_count;

            let channels: Vec<ChannelStatus> = topic.channels.into_iter().map(|channel| ChannelStatus::new(topic_name.clone(), channel)).collect();

            let topic_status = HostTopicStatus {
                topic_name: topic_name.clone(),
                depth,
                message_count, 
                channels
            };

            result.push(topic_status);
        }

        let mut locked_status = self.status.write().unwrap();
        locked_status.clear();
        locked_status.extend(result);

        pb.inc();
    }
}

#[derive(Debug, Serialize)]
struct HostTopicStatus {
    pub topic_name: String,
    pub depth: u64,
    pub message_count: u64,
    pub channels: Vec<ChannelStatus>
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
        let consumers: Vec<ConsumerHost> = channel.clients.iter().map(|x| ConsumerHost::new(x)).collect();
        ChannelStatus {
            channel_name: channel.channel_name,
            topic_name,
            depth: channel.depth,
            in_flight_count: channel.in_flight_count,
            message_count: channel.message_count,
            consumers
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
struct ConsumerHost {
    hostname: String
}

impl ConsumerHost {
    fn new(details: &ClientDetails) -> Self {
        ConsumerHost {
            hostname: details.hostname.clone()
        }
    }
}