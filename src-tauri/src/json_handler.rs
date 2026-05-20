use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

pub struct JSONHandler;

impl JSONHandler {
    pub fn read_from_json<T: DeserializeOwned>(
        file_path: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let file = File::open(file_path)
            .map_err(|e| format!("Failed to open '{}': {}", file_path, e))?;

        let reader = BufReader::new(file);

        let data = serde_json::from_reader(reader)
            .map_err(|e| format!("Failed to parse JSON in '{}': {}", file_path, e))?;

        Ok(data)
    }

    pub fn write_json_file<T: Serialize>(
        file_path: &str,
        data: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(file_path);

        // Write to a temp file alongside the target first.
        // Append ".tmp" so repos.json → repos.json.tmp, not repos.tmp.
        let mut tmp_os = path.as_os_str().to_owned();
        tmp_os.push(".tmp");
        let tmp_path = PathBuf::from(tmp_os);

        {
            let file = File::create(&tmp_path)
                .map_err(|e| format!("Failed to create temp file '{}': {}", tmp_path.display(), e))?;

            let writer = BufWriter::new(file);

            serde_json::to_writer_pretty(writer, data)
                .map_err(|e| format!("Failed to serialize JSON to '{}': {}", tmp_path.display(), e))?;
        }

        // Atomically replace the target with the temp file only after a
        // successful write. This prevents data loss if serialization fails
        // or the process is killed mid-write.
        fs::rename(&tmp_path, path).map_err(|e| {
            // Best-effort cleanup of the temp file if rename fails.
            let _ = fs::remove_file(&tmp_path);
            format!(
                "Failed to finalize write to '{}': {}",
                file_path, e
            )
        })?;

        Ok(())
    }
}
