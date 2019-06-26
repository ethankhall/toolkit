use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use clap::ArgMatches;
use crossbeam_channel::bounded;
use crossbeam_channel::Receiver;

use crate::commands::nsq::api::*;
use crate::commands::progress::*;
use crate::commands::CliError;

const RATE_LIMIT: &str = "200";

static THREADS_RUNNING: AtomicBool = AtomicBool::new(false);
static ERRORS: AtomicUsize = AtomicUsize::new(0);
static SENT: AtomicUsize = AtomicUsize::new(0);
static OFFSET: AtomicUsize = AtomicUsize::new(0);

static API_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static API_DEPTH: AtomicUsize = AtomicUsize::new(0);

struct NsqOptions {
    offset: usize,
    limit: usize,
    rate: f64,
    nsq_lookup: String,
    topic: String,
    file: PathBuf,
    max_depth: usize,
}

impl NsqOptions {
    fn new(matches: &ArgMatches) -> Self {
        let file_name = matches.value_of("INPUT").unwrap();
        let dest_topic = matches.value_of("TOPIC").unwrap();

        let nsq_lookup_host = matches.value_of("nsq_lookup_host").unwrap();
        let nsq_lookup_port = matches.value_of("nsq_lookup_port").unwrap();

        let max_depth = matches
            .value_of("max_depth")
            .map(|x| x.parse::<i32>().unwrap())
            .unwrap_or_else(|| 0);

        let max_depth = if max_depth > 1000 {
            1000
        } else if max_depth < 0 {
            1
        } else {
            max_depth as usize
        };

        let number_of_lines = get_number_of_lines(file_name);
        let raw_rate = matches
            .value_of("rate")
            .unwrap_or_else(|| RATE_LIMIT)
            .parse::<f64>()
            .unwrap();

        let offset = matches
            .value_of("offset")
            .map(|x| x.parse::<usize>().unwrap())
            .unwrap_or_else(|| 0);

        let mut limit = matches
            .value_of("limit")
            .map(|x| x.parse::<usize>().unwrap())
            .unwrap_or_else(|| number_of_lines);

        if limit > number_of_lines {
            limit = number_of_lines;
        }

        info!("Processing {}", file_name);

        NsqOptions {
            offset,
            limit,
            rate: raw_rate,
            nsq_lookup: format!("{}:{}", nsq_lookup_host, nsq_lookup_port),
            topic: s!(dest_topic),
            file: PathBuf::from(file_name),
            max_depth,
        }
    }
}

fn get_number_of_lines(filename: &str) -> usize {
    let reader = crate::commands::file::open_file(filename).expect("To be able to open file");
    let buf_reader = BufReader::new(reader);
    buf_reader.lines().count() as usize
}

pub fn do_send_command(args: &ArgMatches) -> Result<(), CliError> {
    let options = NsqOptions::new(args);

    let (capacity, interval) = if options.rate < 1.0 {
        let dur = Duration::from_secs((1.0 / options.rate) as u64);
        (1 as u32, dur)
    } else {
        (
            options.rate as u32,
            Duration::new(1, 0) / options.rate as u32,
        )
    };

    let status = NsqState::new(
        &options.nsq_lookup,
        NsqFilter::Topic {
            topics: vec![options.topic.clone()].into_iter().collect(),
        },
    );

    debug!("Capacity of in messages: {}", capacity);
    debug!("Interval of new tokens: {:?}", interval);

    let mut ratelimit = ratelimit::Builder::new()
        .capacity(capacity) //number of tokens the bucket will hold
        .interval(interval) //add rate / second
        .build();

    THREADS_RUNNING.store(true, Ordering::SeqCst);

    let base_addresss = match status.get_topic_url(&options.topic.clone()) {
        Some(address) => address,
        None => {
            error!("NSQ does now know about topic {}", options.topic);
            return Err(CliError::new("Unable to get NSQ Host", 2));
        }
    };

    debug!("Using {} as the base url", base_addresss);

    let submit_url = format!("{}/pub?topic={}", base_addresss, &options.topic);

    let pb = ProgressBarHelper::new(ProgressBarType::SizedProgressBar(
        options.limit,
        "[{elapsed_precise}] {bar:80.cyan/blue} {pos:>7}/{len:7} {msg}",
    ));

    let (s1, r1) = bounded(20);

    let mut threads = Vec::new();

    for _ in 0..5 {
        let reciever = r1.clone();
        let url = submit_url.clone();
        threads.push(thread::spawn(move || {
            process_messages(reciever, url);
        }));
    }

    let topic = format!("{}", options.topic);
    do_api_check(&topic, &status);
    threads.push(thread::spawn(move || check_api_status(&topic, &status)));

    let reader = crate::commands::file::open_file(options.file.to_str().unwrap())?;
    let reader = BufReader::new(reader);

    let mut counter = 0;
    for line in reader.lines() {
        loop {
            let max_depth = API_DEPTH.load(Ordering::SeqCst);
            let in_flight = API_IN_FLIGHT.load(Ordering::SeqCst);
            pb.set_message(&format!(
                "In Progress: {:4}\tBacklog Size: {:4}\tOffset: {}",
                in_flight,
                max_depth,
                OFFSET.load(Ordering::SeqCst)
            ));

            if max_depth < options.max_depth {
                break;
            } else {
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        if counter >= options.limit {
            break;
        } else {
            counter += 1;
        }

        ratelimit.wait();
        pb.inc();

        if options.offset > counter {
            OFFSET.fetch_add(1, Ordering::SeqCst);
            continue;
        }

        if s1.send(line.unwrap()).is_err() {
            ERRORS.fetch_add(1, Ordering::SeqCst);
        }
    }
    pb.done();

    THREADS_RUNNING.store(false, Ordering::SeqCst);

    thread::sleep(Duration::from_millis(100));
    for thread in threads {
        thread.join().unwrap();
    }

    println!("========== REPORT ==========");
    println!(
        "Send {} messages, with {} errors.",
        SENT.load(Ordering::SeqCst),
        ERRORS.load(Ordering::SeqCst)
    );
    println!();
    let _ = io::stdout().flush();
    thread::sleep(Duration::from_millis(100));

    return Ok(());
}

fn check_api_status(topic: &str, state: &NsqState) {
    loop {
        if !THREADS_RUNNING.load(Ordering::SeqCst) {
            return;
        }

        do_api_check(topic, state);
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn do_api_check(topic: &str, state: &NsqState) {
    let snapshot = state.update_status();
    let agg = snapshot.topics.get(topic).unwrap().producer_aggregate();
    let max_depth = std::cmp::max(0, agg.depth) as usize;
    API_DEPTH.store(max_depth, Ordering::SeqCst);
}

fn process_messages(reciever: Receiver<String>, path: String) {
    let client = reqwest::Client::new();
    loop {
        match reciever.recv_timeout(Duration::from_millis(100)) {
            Err(_) => {
                if !THREADS_RUNNING.load(Ordering::SeqCst) {
                    return;
                }
                thread::sleep(Duration::from_millis(500));
            }
            Ok(string) => {
                let request = client.post(&path.clone()).body(string).build().unwrap();
                let res = client.execute(request);
                if res.is_err() {
                    error!("Unable to write to the bus! {:#?}", res);
                    ERRORS.fetch_add(1, Ordering::SeqCst);
                } else {
                    OFFSET.fetch_add(1, Ordering::SeqCst);
                    SENT.fetch_add(1, Ordering::SeqCst);
                };
            }
        }
    }
}
