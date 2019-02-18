use clap::ArgMatches;
use rusoto_core::region::Region;
use rusoto_s3::*;
use regex::Regex;

use crate::commands::CliError;

pub fn do_s3_copy(args: &ArgMatches) -> Result<(), CliError> {

    let region = Region::default();
    // construct new S3 client
    let s3 = S3Client::new(region);

    let mut list_obj = ListObjectsV2Request::default();
    list_obj.bucket = s!("");

    // s3.list_objects_v2(list_obj)

    Ok(())
}

fn parse_s3_path(path: &str) -> Option<(String, String)> {
    let regex = Regex::new("s3:/(?P<bucket>[[:word:]]+)/(?P<path>.*)").unwrap();

    let value = match regex.captures(path) {
        None => return None,
        Some(value) => value
    };

    return Some((s!(value.name("bucket").unwrap().as_str()), s!(value.name("path").unwrap().as_str())));
}

#[test]
fn test_parse_path() {
    let (bucket, path) = parse_s3_path("s3:/foo/bar/baz/flig/*").unwrap();

    assert_eq!("foo", bucket);
    assert_eq!("bar/baz/flig/*", path);
}