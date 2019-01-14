use std::fs::File;
use std::io::prelude::*;

pub mod har;

pub trait ToJson {
    fn to_json(self) -> String;
}

pub trait ToMarkdown {
    fn to_markdown(self) -> String;
}

pub trait ToHtml {
    fn to_html(self) -> String;
}

pub struct StdOutWriter {}

impl StdOutWriter {
    pub fn new() -> StdOutWriter {
        return StdOutWriter {};
    }

    pub fn save(self, value: String) -> Result<(), i32> {
        println!("{}", value);
        return Ok(());
    }
}

pub struct FileWriter {
    pub path: String,
}

impl FileWriter {
    pub fn new(path: String) -> FileWriter {
        return FileWriter { path };
    }

    pub fn save(self, value: String) -> Result<(), i32> {
        let bytes = value.as_bytes();

        let mut file = match File::create(self.path.clone()) {
            Ok(file) => file,
            Err(err) => {
                error!(
                    "Unable to create file {} because {}!",
                    self.path.clone(),
                    err
                );
                return Err(2);
            }
        };

        match file.write_all(bytes) {
            Ok(_) => {}
            Err(err) => {
                error!("Unable to write to file ({})!", err);
                return Err(3);
            }
        };

        return Ok(());
    }
}

pub enum Writer {
    StdOut(StdOutWriter),
    File(FileWriter),
}
