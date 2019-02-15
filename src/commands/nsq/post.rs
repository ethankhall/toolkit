use clap::ArgMatches;
use std::fs::File;
use std::path::PathBuf;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering, ATOMIC_BOOL_INIT, ATOMIC_USIZE_INIT};
use std::thread;
use std::time::Duration;

use crossbeam_channel::bounded;
use crossbeam_channel::Receiver;

use indicatif::{ProgressBar, ProgressStyle};

use crate::commands::CliError;

const RATE_LIMIT: &str = "200";

static THREADS_RUNNING: AtomicBool = ATOMIC_BOOL_INIT;
static ERRORS: AtomicUsize = ATOMIC_USIZE_INIT;
static SENT: AtomicUsize = ATOMIC_USIZE_INIT;
static OFFSET: AtomicUsize = ATOMIC_USIZE_INIT;

struct NsqOptions {
    offset: usize,
    limit: usize,
    rate: usize,
    nsq_lookup: String,
    topic: String,
    file: PathBuf
}

impl NsqOptions {
    fn new(matches: &ArgMatches) -> Self {
        let file_name = matches.value_of("INPUT").unwrap();
        let dest_topic = matches.value_of("TOPIC").unwrap();

        let nsq_lookup_host = matches.value_of("nsq_lookup_host").unwrap();
        let nsq_lookup_port = matches.value_of("nsq_lookup_port").unwrap();

        let number_of_lines = get_number_of_lines(file_name);
        let rate = matches
            .value_of("rate")
            .unwrap_or_else(|| RATE_LIMIT)
            .parse::<usize>()
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

        NsqOptions { offset, limit, rate, nsq_lookup: format!("{}:{}", nsq_lookup_host, nsq_lookup_port), topic: s!(dest_topic), file: PathBuf::from(file_name)}
    }
}

fn get_number_of_lines(filename: &str) -> usize {
    let f = File::open(filename).expect("To be able to open file");
    let file = BufReader::new(&f);
    file.lines().count() as usize
}

pub fn do_send_command(args: &ArgMatches) -> Result<(), CliError> {
    let options = NsqOptions::new(args);

    let mut ratelimit = ratelimit::Builder::new()
        .capacity(options.rate as u32) //number of tokens the bucket will hold
        .frequency(options.rate as u32) //add rate / second
        .build();

    THREADS_RUNNING.store(true, Ordering::SeqCst);

    let handle = ratelimit.make_handle();
    thread::spawn(move || ratelimit.run());

    let base_url = match super::api::get_base_url_for_topic(&options.nsq_lookup, &options.topic) {
        Some(url) => url,
        None => { return Err(CliError::new("Unable to get NSQ Host", 2)) }
    };

    let submit_url = format!("{}/pub?topic={}", base_url, &options.topic);

    let style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:80.cyan/blue} {pos:>7}/{len:7} {msg}")
        .progress_chars("##-");
    let progress_bar = ProgressBar::new(options.limit as u64);
    progress_bar.set_style(style.clone());

    let (s1, r1) = bounded(20);

      let mut threads = Vec::new();

    for _ in 0..10 {
        let reciever = r1.clone();
        let url = submit_url.clone();
        let mut limiter = handle.clone();
        threads.push(thread::spawn(move || {
            process_messages(reciever, url, &mut limiter);
        }));
    }


    let f = File::open(options.file)?;
    let file = BufReader::new(&f);
    let mut counter = 0;
    for line in file.lines() {
        if counter >= options.limit {
            break;
        } else {
            counter += 1;
        }

        progress_bar.inc(1);

        if options.offset > counter {
            OFFSET.fetch_add(1, Ordering::SeqCst);
            continue;
        }

        if s1.send(line.unwrap()).is_err() {
            ERRORS.fetch_add(1, Ordering::SeqCst);
        }

        if counter % 100 == 0 {
            loop {
                if let Some((max_depth, in_flight)) = super::api::get_queue_size(&base_url, &options.topic) {
                    progress_bar.set_message(&format!(
                        "In Progress: {:4}\tBacklog Size: {:4}\tOffset: {}",
                        in_flight,
                        max_depth,
                        OFFSET.load(Ordering::SeqCst)
                    ));
                    if max_depth < 100 {
                        break;
                    } else {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
    }
    progress_bar.finish();

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

fn process_messages(reciever: Receiver<String>, path: String, ratelimit: &mut ratelimit::Handle) {
    let client = reqwest::Client::new();
    loop {
        match reciever.recv_timeout(Duration::from_millis(100)) {
            Err(_) => {
                if !THREADS_RUNNING.load(Ordering::SeqCst) {
                    return;
                }
            }
            Ok(string) => {
                ratelimit.wait();
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