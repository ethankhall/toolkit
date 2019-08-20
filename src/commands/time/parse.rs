use chrono::naive::{NaiveDate, NaiveDateTime};
use chrono::{DateTime, Datelike, FixedOffset, Offset, TimeZone, Timelike};
use chrono_tz::Tz;
use regex::{Captures, Regex};
use std::str::FromStr;

#[cfg(test)]
use chrono::Duration;

lazy_static! {
    static ref CALENDAR_DAY: Regex = Regex::new(r"(?P<p1>\d{1,4})[-\\/](?P<p2>\d{1,4})[-\\/](?P<p3>\d{1,4})(?P<t>T)?").unwrap();
    static ref TIME: Regex = Regex::new(r"(?i)(?P<hour>\d{1,2}):(?P<min>\d{2})([:\.](?P<sec>\d{2})?(\.(?P<nano>\d+))?)?( ?(?P<format>(.m)))?").unwrap();
    static ref TIME_ZONE: Regex = Regex::new(r"(?P<zone>[\+-]\d{2}(:?\d{2})?)").unwrap();
}

const SECONDS_MAX: u64 = 10_000_000_000;
const MILLI_PER_SEC: u64 = 1_000;
const NANO_PER_SEC: u64 = 1_000_000_000;
const MILLI_PER_NANO: u64 = 1_000_000;

#[derive(Debug, PartialEq)]
pub struct StringTime {
    dates: Vec<CalendarDate>,
    time: Option<CalendarTime>,
    timezone: Option<FixedOffset>,
}

impl StringTime {
    fn is_empty(&self) -> bool {
        self.dates.is_empty() && self.time.is_none() && self.timezone.is_none()
    }

    pub fn to_utc_date_time(&self) -> DateTime<chrono::Utc> {
        use chrono::naive::NaiveTime;
        use chrono::{Local, Utc};

        let date = self
            .dates
            .get(0)
            .map(|x| NaiveDate::from_ymd(x.year as i32, x.month, x.day))
            .unwrap_or_else(|| Local::now().naive_local().date());

        let time = self
            .time
            .clone()
            .map(|x| NaiveTime::from_hms_nano(x.hour, x.min, x.second, x.nano as u32))
            .unwrap_or_else(|| Local::now().naive_local().time());

        let timezone = self.timezone.unwrap_or_else(|| FixedOffset::east(0));

        let naive_datetime = NaiveDateTime::new(date, time);

        let datetime: DateTime<FixedOffset> = DateTime::from_utc(naive_datetime, timezone);

        datetime.with_timezone(&Utc)
    }

    pub fn make_permutations(&self) -> Vec<DateTime<FixedOffset>> {
        use chrono::naive::NaiveTime;
        use chrono::{Date, Local};

        let time = self
            .time
            .clone()
            .map(|x| NaiveTime::from_hms_nano(x.hour, x.min, x.second, x.nano as u32))
            .unwrap_or_else(|| Local::now().naive_local().time());

        let timezone = self.timezone.unwrap_or_else(|| FixedOffset::east(0));

        let mut dates: Vec<Date<FixedOffset>> = self
            .dates
            .iter()
            .map(|x| NaiveDate::from_ymd(x.year as i32, x.month, x.day))
            .map(|x| Date::from_utc(x, timezone))
            .collect();

        if dates.is_empty() {
            dates.push(Local::now().date().with_timezone(&timezone));
        }

        dates
            .into_iter()
            .map(|x| x.and_time(time).unwrap())
            .collect()
    }
}

#[derive(Debug, PartialEq)]
pub enum TimeResult {
    Epoch(EpochTime),
    String(StringTime),
}

#[derive(Debug, PartialEq)]
pub enum EpochTime {
    Seconds(u64),
    Nanoseconds(u64, u64),
}

impl EpochTime {
    pub fn to_utc_date_time(&self) -> DateTime<chrono::Utc> {
        use chrono::Utc;

        let date = match self {
            EpochTime::Seconds(s) => NaiveDateTime::from_timestamp(*s as i64, 0),
            EpochTime::Nanoseconds(sec, nano) => {
                NaiveDateTime::from_timestamp(*sec as i64, *nano as u32)
            }
        };

        DateTime::from_utc(date, Utc)
    }

    pub fn make_permutations(&self) -> Vec<DateTime<FixedOffset>> {
        vec![self.to_utc_date_time().with_timezone(&FixedOffset::east(0))]
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CalendarDate {
    year: u32,
    month: u32,
    day: u32,
}

impl CalendarDate {
    fn new(year: u32, month: u32, day: u32) -> Self {
        CalendarDate { year, month, day }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CalendarTime {
    hour: u32,
    min: u32,
    second: u32,
    nano: u128,
}

impl CalendarTime {
    fn new(hour: u32, min: u32, second: u32, nano: u128) -> Self {
        CalendarTime {
            hour,
            min,
            second,
            nano,
        }
    }
}

pub fn parse(input: &str) -> Result<TimeResult, String> {
    use chrono::Local;

    let mut input = s!(input);
    if let Ok(value) = input.parse::<u64>() {
        return parse_number(value);
    }

    let mut string_time = StringTime {
        dates: Vec::new(),
        time: None,
        timezone: None,
    };

    if "now" == input {
        let now = Local::now();
        string_time
            .dates
            .push(CalendarDate::new(now.year() as u32, now.month(), now.day()));
        string_time.time = Some(CalendarTime::new(
            now.hour(),
            now.minute(),
            now.second(),
            now.nanosecond() as u128,
        ));
        string_time.timezone = Some(now.offset().fix());

        return Ok(TimeResult::String(string_time));
    }

    if let Some(value) = CALENDAR_DAY.captures(&input) {
        string_time.extract_dates(&value);

        input = input.replace(CALENDAR_DAY.find(&input).unwrap().as_str(), "");
    }

    for try_tz in input.split(" ") {
        let stripped = try_tz.replace("[", "").replace("]", "");
        let stripped = stripped.as_str();
        let matches = match stripped {
            "PST" | "PDT" => Some(Tz::PST8PDT),
            "MST" | "MDT" => Some(Tz::MST7MDT),
            "EST" | "EDT" => Some(Tz::EST5EDT),
            "CST" | "CDT" => Some(Tz::CST6CDT),
            _ => Tz::from_str(stripped).ok(),
        };

        if let Some(parsed_tz) = matches {
            if let Some(date) = string_time.dates.first() {
                string_time.timezone = Some(
                    parsed_tz
                        .ymd(date.year as i32, date.month, date.day)
                        .offset()
                        .fix(),
                );
                input = input.replace(try_tz, "");
                break;
            }
        }
    }

    if let Some(value) = TIME.captures(&input) {
        string_time.extract_time(&value);

        input = input.replace(TIME.find(&input).unwrap().as_str(), "");
    }

    if let Some(value) = TIME_ZONE.captures(&input) {
        string_time.extract_time_zone(&value);

        input = input.replace(TIME_ZONE.find(&input).unwrap().as_str(), "");
    }

    if string_time.is_empty() {
        Err(format!("Unknown format {}", input))
    } else {
        Ok(TimeResult::String(string_time))
    }
}

impl StringTime {
    fn extract_time_zone(&mut self, value: &Captures) {
        let zone_str = value.name("zone").unwrap().as_str().replace(":", "");
        let zone = &zone_str[1..];

        let mut zone_offset = if zone.len() == 2 {
            zone.parse::<i32>().unwrap() * 100
        } else {
            zone.parse::<i32>().unwrap()
        };

        if zone_str.chars().nth(0).unwrap() == '-' {
            zone_offset = -1 * zone_offset;
        }

        let seconds_offset = (zone_offset % 100) * 60 + (zone_offset / 100) * 60 * 60;

        self.timezone = Some(FixedOffset::east(seconds_offset));
    }

    fn extract_time(&mut self, value: &Captures) {
        let mut hour = value.name("hour").unwrap().as_str().parse::<u32>().unwrap();
        let min = value.name("min").unwrap().as_str().parse::<u32>().unwrap();
        let second = value
            .name("sec")
            .map_or(0, |x| x.as_str().parse::<u32>().unwrap());
        let (length, sub_sec) = value.name("nano").map_or((0, 0), |x| {
            (x.as_str().len(), x.as_str().parse::<u128>().unwrap())
        });

        if let Some(format) = value.name("format") {
            if format.as_str().to_lowercase() == "pm" {
                hour += 12;
            }
        }

        let nano = match length {
            3 => sub_sec * 10u128.pow(6),
            6 => sub_sec * 10u128.pow(3),
            9 => sub_sec,
            _ => 0,
        };

        self.time = Some(CalendarTime::new(hour, min, second, nano));
    }

    fn extract_dates(&mut self, value: &Captures) {
        let part1 = value.name("p1").unwrap().as_str().parse::<u32>().unwrap();
        let part2 = value.name("p2").unwrap().as_str().parse::<u32>().unwrap();
        let part3 = value.name("p3").unwrap().as_str().parse::<u32>().unwrap();

        let year = if part1 < 100 { part1 + 2000 } else { part1 };

        if value.name("t").is_some() {
            if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part2, part3) {
                self.dates.push(CalendarDate {
                    year,
                    month: part2,
                    day: part3,
                });
            }
            return;
        }

        if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part3, part2) {
            self.dates.push(CalendarDate {
                year,
                month: part3,
                day: part2,
            });
        }

        if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part2, part3) {
            self.dates.push(CalendarDate {
                year,
                month: part2,
                day: part3,
            });
        }

        let year = if part3 < 100 { part3 + 2000 } else { part3 };

        if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part1, part2) {
            self.dates.push(CalendarDate {
                year: year,
                month: part1,
                day: part2,
            });
        }

        if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part2, part1) {
            self.dates.push(CalendarDate {
                year: year,
                month: part2,
                day: part1,
            });
        }
    }
}

fn parse_number(input: u64) -> Result<TimeResult, String> {
    return if input < SECONDS_MAX {
        Ok(TimeResult::Epoch(EpochTime::Seconds(input)))
    } else if input < SECONDS_MAX * MILLI_PER_SEC {
        let seconds = input / MILLI_PER_SEC;
        let millis = input % MILLI_PER_SEC;
        Ok(TimeResult::Epoch(EpochTime::Nanoseconds(
            seconds,
            millis * MILLI_PER_NANO,
        )))
    } else if input < SECONDS_MAX * NANO_PER_SEC {
        let seconds = input / NANO_PER_SEC;
        let nanos = input % NANO_PER_SEC;
        Ok(TimeResult::Epoch(EpochTime::Nanoseconds(seconds, nanos)))
    } else {
        Err(format!("Unknown number {}", input))
    };
}

#[cfg(test)]
fn assert_contains_date(time_results: &TimeResult, required: &[CalendarDate]) {
    let string_time = match time_results {
        TimeResult::String(string_time) => string_time,
        _ => panic!("Was epoch"),
    };

    let mut dates = string_time.dates.clone();

    for ut in required {
        assert!(
            dates.contains(ut),
            "{:?} was not found in results: {:?}",
            ut,
            dates
        );
        dates.iter().position(|item| ut == item).map(|i| dates.remove(i));
    }

    assert_eq!(0, dates.len());
}

#[cfg(test)]
fn assert_time(time_results: &TimeResult, required: CalendarTime) {
    let string_time = match time_results {
        TimeResult::String(string_time) => string_time,
        _ => panic!("Was epoch"),
    };

    assert_eq!(Some(required), string_time.time);
}

#[cfg(test)]
fn assert_time_zone(time_results: &TimeResult, required: FixedOffset) {
    let string_time = match time_results {
        TimeResult::String(string_time) => string_time,
        _ => panic!("Was epoch"),
    };

    let date = NaiveDate::from_ymd(2019, 2, 3);
    let required_offset = required.offset_from_utc_date(&date).fix();
    let testing_offset = string_time
        .timezone
        .unwrap()
        .offset_from_utc_date(&date)
        .fix();

    assert_eq!(
        required_offset.local_minus_utc(),
        testing_offset.local_minus_utc(),
        "{:?} != {:?}",
        required_offset,
        testing_offset
    );
}

#[cfg(test)]
fn assert_full_output(
    time_results: &TimeResult,
    dates: &[CalendarDate],
    time: CalendarTime,
    offset: FixedOffset,
) {
    assert_contains_date(time_results, dates);
    assert_time(time_results, time);
    assert_time_zone(time_results, offset);
}

#[cfg(test)]
fn parse_unwrap(input: &str) -> TimeResult {
    parse(input).unwrap()
}

#[test]
fn parse_epoch_timestamps_samples() {
    assert_eq!(
        TimeResult::Epoch(EpochTime::Seconds(1554248133)),
        parse_unwrap("1554248133")
    );
    assert_eq!(
        TimeResult::Epoch(EpochTime::Nanoseconds(1554248133, 358000000)),
        parse_unwrap("1554248133358")
    );
    assert_eq!(
        TimeResult::Epoch(EpochTime::Nanoseconds(1555438653, 801529000)),
        parse_unwrap("1555438653801529000")
    );
}

#[test]
fn parse_dates_samples() {
    assert_contains_date(
        &parse_unwrap("2019/12/15"),
        &[CalendarDate::new(2019, 12, 15)],
    );
    assert_contains_date(
        &parse_unwrap("2019-12-15"),
        &[CalendarDate::new(2019, 12, 15)],
    );
    assert_contains_date(
        &parse_unwrap("2019\\12\\15"),
        &[CalendarDate::new(2019, 12, 15)],
    );

    assert_contains_date(
        &parse_unwrap("2019/15/12"),
        &[CalendarDate::new(2019, 12, 15)],
    );
    assert_contains_date(
        &parse_unwrap("2019-15-12"),
        &[CalendarDate::new(2019, 12, 15)],
    );
    assert_contains_date(
        &parse_unwrap("2019\\15\\12"),
        &[CalendarDate::new(2019, 12, 15)],
    );

    assert_contains_date(
        &parse_unwrap("2019/03/04"),
        &[CalendarDate::new(2019, 4, 3), CalendarDate::new(2019, 3, 4)],
    );
    assert_contains_date(
        &parse_unwrap("2019\\03\\04"),
        &[CalendarDate::new(2019, 4, 3), CalendarDate::new(2019, 3, 4)],
    );
    assert_contains_date(
        &parse_unwrap("2019-03-04"),
        &[CalendarDate::new(2019, 4, 3), CalendarDate::new(2019, 3, 4)],
    );

    assert_contains_date(
        &parse_unwrap("4-3-19"),
        &[
            CalendarDate::new(2019, 4, 3),
            CalendarDate::new(2019, 3, 4),
            CalendarDate::new(2004, 3, 19),
        ],
    );

    assert_contains_date(&parse_unwrap("4-13-19"), &[CalendarDate::new(2019, 4, 13)]);

    assert_contains_date(
        &parse_unwrap("13-4-19"),
        &[
            CalendarDate::new(2019, 4, 13),
            CalendarDate::new(2013, 4, 19),
        ],
    );
}

#[test]
fn parse_times_samples() {
    assert_time(&parse_unwrap("10:30"), CalendarTime::new(10, 30, 0, 0));
    assert_time(&parse_unwrap("10:30:45"), CalendarTime::new(10, 30, 45, 0));
    assert_time(
        &parse_unwrap("10:30:45.123"),
        CalendarTime::new(10, 30, 45, 123_000_000),
    );
    assert_time(
        &parse_unwrap("10:30:45.123456"),
        CalendarTime::new(10, 30, 45, 123_456_000),
    );
    assert_time(
        &parse_unwrap("10:30:45.123456789"),
        CalendarTime::new(10, 30, 45, 123_456_789),
    );
    assert_time(
        &parse_unwrap("10:30:45.123456789 am"),
        CalendarTime::new(10, 30, 45, 123_456_789),
    );
    assert_time(
        &parse_unwrap("10:30:45.123456789 AM"),
        CalendarTime::new(10, 30, 45, 123_456_789),
    );
    assert_time(
        &parse_unwrap("10:30:45.123456789 pm"),
        CalendarTime::new(22, 30, 45, 123_456_789),
    );
    assert_time(
        &parse_unwrap("10:30:45.123456789 PM"),
        CalendarTime::new(22, 30, 45, 123_456_789),
    );
}

#[test]
fn test_timezone() {
    assert!(TIME_ZONE.is_match("+12"));
    assert!(TIME_ZONE.is_match("+1200"));
    assert!(TIME_ZONE.is_match("+12:00"));
    assert!(TIME_ZONE.is_match("-12:00"));

    let twelve_hours = 12 * 60 * 60;
    let ten_hours = 10 * 60 * 60;

    assert_time_zone(&parse_unwrap("10:30+1200"), FixedOffset::east(twelve_hours));
    assert_time_zone(
        &parse_unwrap("10:30+12:00"),
        FixedOffset::east(twelve_hours),
    );
    assert_time_zone(
        &parse_unwrap("10:30-12:00"),
        FixedOffset::west(twelve_hours),
    );
    assert_time_zone(&parse_unwrap("10:30+12"), FixedOffset::east(twelve_hours));
    assert_time_zone(&parse_unwrap("10:30+0000"), FixedOffset::west(0));
    assert_time_zone(&parse_unwrap("10:30-10"), FixedOffset::west(ten_hours));
}

#[test]
fn test_real_examples() {
    assert_full_output(
        &parse_unwrap("2018-12-04T04:20:22.205838800+00:00"),
        &[CalendarDate::new(2018, 12, 4)],
        CalendarTime::new(4, 20, 22, 205_838_800),
        FixedOffset::east(0),
    );
    assert_full_output(
        &parse_unwrap("2019-02-08T08:00:00+0000"),
        &[CalendarDate::new(2019, 2, 8)],
        CalendarTime::new(8, 0, 0, 0),
        FixedOffset::east(0),
    );
    assert_full_output(
        &parse_unwrap("2019-02-08T08:00:00 PST"),
        &[CalendarDate::new(2019, 2, 8)],
        CalendarTime::new(8, 0, 0, 0),
        FixedOffset::west(Duration::hours(8).num_seconds() as i32),
    );
    assert_full_output(
        &parse_unwrap("2019-02-08T08:00:00 America/Los_Angeles"),
        &[CalendarDate::new(2019, 2, 8)],
        CalendarTime::new(8, 0, 0, 0),
        FixedOffset::west(Duration::hours(8).num_seconds() as i32),
    );
    assert_full_output(
        &parse_unwrap("2019-02-08T08:00:00 [America/Los_Angeles]"),
        &[CalendarDate::new(2019, 2, 8)],
        CalendarTime::new(8, 0, 0, 0),
        FixedOffset::west(Duration::hours(8).num_seconds() as i32),
    );
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::io::BufRead;
    use std::io::BufReader;
    use std::path::PathBuf;

    #[test]
    fn test_parsing_inputs() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/time_known_formats.txt");

        let f = File::open(d).unwrap();
        let file = BufReader::new(&f);
        for line in file.lines() {
            let l = line.unwrap();
            println!("Validating {} as input.", l);
            if let Err(s) = parse(&l) {
                panic!("Unable to parse {}", s)
            }
        }
    }
}
