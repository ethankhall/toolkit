mod latest;
mod sql;

pub use self::latest::do_json_latest_command;
pub use self::sql::do_json_sql_command;

use json::JsonValue;

// https://github.com/ms705/nom-sql

fn find_field<'a>(field: &Vec<String>, json_input: &'a JsonValue) -> Option<&'a JsonValue> {
    let mut value = json_input;

    for part in field {
        value = &value[part];
        if value.is_null() {
            return None;
        }
    }

    return Some(value);
}

fn parse_path(path: &str) -> Vec<String> {
    let path = if path.starts_with(".") {
        path.replacen(".", "", 1)
    } else {
        path.to_string()
    };

    let split: Vec<String> = path.split_terminator(".").map(|x| x.to_string()).collect();

    return split;
}
