use regex::{Captures, Regex};
use chrono::naive::NaiveDate;

lazy_static! {
    static ref CALENDAR_DAY: Regex = Regex::new(r"(?P<p1>\d{1,4})[-\\/](?P<p2>\d{1,4})[-\\/](?P<p3>\d{1,4})").unwrap();
    static ref TIME: Regex = Regex::new(r"(?i)(?P<hour>\d{1,2}):(?P<min>\d{2})([:\.](?P<sec>\d{2})?(\.(?P<nano>\d+))?)?( ?(?P<format>(.m)))?").unwrap();
    static ref TIME_ZONE: Regex = Regex::new(r"(?P<zone>[\+-]\d{2}(:?\d{2})?)").unwrap();
}

#[derive(Debug, PartialEq)]
pub struct StringTime {
    dates: Vec<CalendarDate>,
    time: Option<CalendarTime>,
    timezone: Option<i32>
}

impl StringTime {
    fn is_empty(&self) -> bool {
        self.dates.is_empty() && self.time.is_none() && self.timezone.is_none()
    }
}
 
#[derive(Debug, PartialEq)]
pub enum TimeResult {
    Unknown(String),
    Epoch(EpochTime),
    String(StringTime)
}

#[derive(Debug, PartialEq)]
pub enum EpochTime {
    Seconds(u128),
    Milliseconds(u128),
    Nanoseconds(u128)
}

#[derive(Debug, PartialEq, Clone)]
pub struct CalendarDate {
    year: u32,
    month: u32,
    day: u32
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
        CalendarTime { hour, min, second, nano}
    }
}

fn parse(input: &str) -> TimeResult {
    let mut input = s!(input);
    if let Ok(value) = input.parse::<u128>() {
        return parse_number(value);
    }

    let mut string_time = StringTime { dates: Vec::new(), time: None, timezone: None };

    if let Some(value) = CALENDAR_DAY.captures(&input) {
        extract_dates(&mut string_time, &value);

        input = input.replace(CALENDAR_DAY.find(&input).unwrap().as_str(), "");
    }

    if let Some(value) = TIME.captures(&input) {
        extract_time(&mut string_time, &value);

        input = input.replace(TIME.find(&input).unwrap().as_str(), "");
    }

    if let Some(value) = TIME_ZONE.captures(&input) {
        extract_time_zone(&mut string_time, &value);

        input = input.replace(TIME_ZONE.find(&input).unwrap().as_str(), "");
    }

    if string_time.is_empty() {
        TimeResult::Unknown(format!("Unknown format {}", input))
    } else {
        TimeResult::String(string_time)
    }
}

fn extract_time_zone<'t>(string_time: &mut StringTime, value: &'t Captures) {
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

    string_time.timezone = Some(zone_offset);
}

fn extract_time<'t>(string_time: &mut StringTime, value: &'t Captures) {
    let mut hour = value.name("hour").unwrap().as_str().parse::<u32>().unwrap();
    let min = value.name("min").unwrap().as_str().parse::<u32>().unwrap();
    let second = value.name("sec").map_or(0, |x| x.as_str().parse::<u32>().unwrap());
    let (length, sub_sec) = value.name("nano").map_or((0, 0), |x| (x.as_str().len(), x.as_str().parse::<u128>().unwrap()));

    if let Some(format) = value.name("format") {
        if format.as_str().to_lowercase() == "pm" {
            hour += 12;
        }
    }

    let nano = match length {
        3 => sub_sec * 10u128.pow(6),
        6 => sub_sec * 10u128.pow(3),
        9 => sub_sec,
        _ => 0
    };

    string_time.time = Some(CalendarTime::new(hour, min, second, nano));
}

fn extract_dates<'t>(string_time: &mut StringTime, value: &'t Captures) {
    let part1 = value.name("p1").unwrap().as_str().parse::<u32>().unwrap();
    let part2 = value.name("p2").unwrap().as_str().parse::<u32>().unwrap();
    let part3 = value.name("p3").unwrap().as_str().parse::<u32>().unwrap();

    let year = if part1 < 100 {
        part1 + 2000
    } else {
        part1
    };

    if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part3, part2) {
        string_time.dates.push(CalendarDate { year, month: part3, day: part2 });
    }

    if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part2, part3) {
        string_time.dates.push(CalendarDate { year, month: part2, day: part3 });
    }

    let year = if part3 < 100 {
        part3 + 2000
    } else {
        part3
    };

    if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part1, part2) {
        string_time.dates.push(CalendarDate { year: year, month: part1, day: part2 });
    }

    if let Some(_) = NaiveDate::from_ymd_opt(year as i32, part2, part1) {
        string_time.dates.push(CalendarDate { year: year, month: part2, day: part1 });
    }
}

fn parse_number(input: u128) -> TimeResult {
    return if input < 2u128.pow(31) - 1 {
        TimeResult::Epoch(EpochTime::Seconds(input))
    } else if input < (2u128.pow(31) - 1) * 1000 {
        TimeResult::Epoch(EpochTime::Milliseconds(input))
    } else if input < (2u128.pow(31) - 1) * 1_000_000 {
        TimeResult::Epoch(EpochTime::Nanoseconds(input))
    } else {
        TimeResult::Unknown(format!("Unknown number {}", input))
    }
}


#[cfg(test)]
fn assert_contains_date(time_results: TimeResult, required: &[CalendarDate]) {
    let string_time = match time_results {
        TimeResult::String(string_time) => string_time,
        _ => { panic!("Was epoch") }
    };

    let mut dates = string_time.dates.clone();

    for ut in required {
        assert!(dates.contains(ut), "{:?} was not found in results", ut);
        dates.remove_item(&ut);
    }

    assert_eq!(0, dates.len());
}

#[cfg(test)]
fn assert_time(time_results: TimeResult, required: CalendarTime) {
    let string_time = match time_results {
        TimeResult::String(string_time) => string_time,
        _ => { panic!("Was epoch") }
    };

    assert_eq!(Some(required), string_time.time);
}

#[cfg(test)]
fn assert_time_zone(time_results: TimeResult, required: i32) {
    let string_time = match time_results {
        TimeResult::String(string_time) => string_time,
        _ => { panic!("Was epoch") }
    };

    assert_eq!(Some(required), string_time.timezone);
}

#[test]
fn parse_epoch_timestamps_samples() {
    assert_eq!(TimeResult::Epoch(EpochTime::Seconds(1554248133)), parse("1554248133"));
    assert_eq!(TimeResult::Epoch(EpochTime::Milliseconds(1554248133358)), parse("1554248133358"));
    assert_eq!(TimeResult::Epoch(EpochTime::Nanoseconds(1554248133358000)), parse("1554248133358000"));
}

#[test]
fn parse_dates_samples() {
    assert_contains_date(parse("2019/12/15"), &[CalendarDate::new(2019, 12, 15)]);
    assert_contains_date(parse("2019-12-15"), &[CalendarDate::new(2019, 12, 15)]);
    assert_contains_date(parse("2019\\12\\15"), &[CalendarDate::new(2019, 12, 15)]);

    assert_contains_date(parse("2019/15/12"), &[CalendarDate::new(2019, 12, 15)]);
    assert_contains_date(parse("2019-15-12"), &[CalendarDate::new(2019, 12, 15)]);
    assert_contains_date(parse("2019\\15\\12"), &[CalendarDate::new(2019, 12, 15)]);

    assert_contains_date(parse("2019/03/04"), &[CalendarDate::new(2019, 4, 3), CalendarDate::new(2019, 3, 4)]);
    assert_contains_date(parse("2019\\03\\04"), &[CalendarDate::new(2019, 4, 3), CalendarDate::new(2019, 3, 4)]);
    assert_contains_date(parse("2019-03-04"), &[CalendarDate::new(2019, 4, 3), CalendarDate::new(2019, 3, 4)]);

    assert_contains_date(parse("4-3-19"), &[CalendarDate::new(2019, 4, 3), CalendarDate::new(2019, 3, 4), CalendarDate::new(2004, 3, 19)]);

    assert_contains_date(parse("4-13-19"), &[CalendarDate::new(2019, 4, 13)]);

    assert_contains_date(parse("13-4-19"), &[CalendarDate::new(2019, 4, 13), CalendarDate::new(2013, 4, 19)]);
}

#[test]
fn parse_times_samples() {
    assert_time(parse("10:30"), CalendarTime::new(10, 30, 0, 0));
    assert_time(parse("10:30:45"), CalendarTime::new(10, 30, 45, 0));
    assert_time(parse("10:30:45.123"), CalendarTime::new(10, 30, 45, 123_000_000));
    assert_time(parse("10:30:45.123456"), CalendarTime::new(10, 30, 45, 123_456_000));
    assert_time(parse("10:30:45.123456789"), CalendarTime::new(10, 30, 45, 123_456_789));
    assert_time(parse("10:30:45.123456789 am"), CalendarTime::new(10, 30, 45, 123_456_789));
    assert_time(parse("10:30:45.123456789 AM"), CalendarTime::new(10, 30, 45, 123_456_789));
    assert_time(parse("10:30:45.123456789 pm"), CalendarTime::new(22, 30, 45, 123_456_789));
    assert_time(parse("10:30:45.123456789 PM"), CalendarTime::new(22, 30, 45, 123_456_789));
}

#[test]
fn test_timezone() {
    assert!(TIME_ZONE.is_match("+12"));
    assert!(TIME_ZONE.is_match("+1200"));
    assert!(TIME_ZONE.is_match("+12:00"));
    assert!(TIME_ZONE.is_match("-12:00"));

    assert_time_zone(parse("10:30+1200"), 1200);
    assert_time_zone(parse("10:30+12:00"), 1200);
    assert_time_zone(parse("10:30-12:00"), -1200);
    assert_time_zone(parse("10:30+12"), 1200);
    assert_time_zone(parse("10:30+0000"), 0);
    assert_time_zone(parse("10:30-10"), -1000);
}