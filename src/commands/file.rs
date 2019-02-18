use std::io::{self, BufReader, Read};
use std::fs::File;

use flate2::read::GzDecoder;

pub fn open_file(path: &str) -> io::Result<Box<dyn Read>> {
    if path == "-" {
        Ok(Box::new(io::stdin()))
    } else if path.ends_with(".gz") {
        let file = File::open(path)?;
        let buf_reader = BufReader::new(file);
 
        Ok(Box::new(GzDecoder::new(buf_reader)))
    } else {
        let file = File::open(path)?;
        Ok(Box::new(BufReader::new(file)))
    }
}