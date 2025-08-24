use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
pub struct JSONHandler {}

impl JSONHandler {
    pub fn read_from_json<T: DeserializeOwned>(
        file_path: String,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let path = Path::new(&file_path);

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let data = serde_json::from_reader(reader)?;

        Ok(data)
    }

    pub fn write_json_file<T: Serialize>(
        file_path: String,
        data: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(&file_path);

        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, data)?;

        Ok(())
    }
}
