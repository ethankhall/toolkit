pub struct CliError {
    message: String,
    pub code: i32,
}

impl CliError {
    fn new<S: Into<String>>(message: S, code: i32) -> Self {
        CliError {
            message: message.into(),
            code,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error! {}", self.message)
    }
}

impl std::convert::From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError {
            message: e.to_string(),
            code: 4,
        }
    }
}

mod file;
pub mod har;
pub mod json;
pub mod nsq;
pub mod time;
