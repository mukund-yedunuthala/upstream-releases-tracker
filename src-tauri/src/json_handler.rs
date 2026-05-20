use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

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
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, data)
            .map_err(|e| format!("Failed to serialize JSON to '{}': {}", tmp_path.display(), e))?;
        // Explicit flush — BufWriter::drop silently discards flush errors.
        writer.flush()
            .map_err(|e| format!("Failed to flush write buffer to '{}': {}", tmp_path.display(), e))?;
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed to finalize write to '{}': {}", file_path, e)
    })?;

    Ok(())
}
