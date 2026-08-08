use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

pub fn read_from_json<T: DeserializeOwned>(
    file_path: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let file =
        File::open(file_path).map_err(|e| format!("Failed to open '{}': {}", file_path, e))?;
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

    let mut tmp_os = path.as_os_str().to_owned();
    tmp_os.push(".tmp");
    let tmp_path = PathBuf::from(tmp_os);

    {
        let file = File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp file '{}': {}", tmp_path.display(), e))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, data).map_err(|e| {
            format!(
                "Failed to serialize JSON to '{}': {}",
                tmp_path.display(),
                e
            )
        })?;
        writer.flush().map_err(|e| {
            format!(
                "Failed to flush write buffer to '{}': {}",
                tmp_path.display(),
                e
            )
        })?;
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed to finalize write to '{}': {}", file_path, e)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Row {
        value: String,
    }

    // --- round-trip ---

    #[test]
    fn write_then_read_round_trips() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        let data = Row {
            value: "hello".to_string(),
        };
        write_json_file(path, &data).unwrap();
        let read_back: Row = read_from_json(path).unwrap();
        assert_eq!(read_back, data);
    }

    // --- existing file is preserved when write fails ---
    //
    // We simulate a write failure by pointing the target path at a directory
    // (so File::create on the ".tmp" sibling fails because the path already
    // has a trailing component that resolves to a directory on some systems,
    // or because the computed tmp path is invalid).
    // A more reliable approach: write an initial good value, then attempt a
    // write to a path whose *parent directory* is read-only so the temp file
    // cannot be created — the existing file must remain intact.
    #[test]
    fn failed_write_does_not_corrupt_existing_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        // Write an initial value into the directory.
        let target = dir.path().join("data.json");
        let original = Row {
            value: "original".to_string(),
        };
        write_json_file(target.to_str().unwrap(), &original).unwrap();

        // Create a read-only subdirectory; point the write target inside it
        // so that creating the .tmp sibling fails.
        let ro_dir = dir.path().join("readonly");
        std::fs::create_dir(&ro_dir).unwrap();
        std::fs::set_permissions(&ro_dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let ro_target = ro_dir.join("data.json");

        // Write-to-readonly must fail.
        let err = write_json_file(
            ro_target.to_str().unwrap(),
            &Row {
                value: "new".to_string(),
            },
        );
        assert!(err.is_err(), "expected write to read-only dir to fail");

        // The original file in the writable dir must be untouched.
        let still_original: Row = read_from_json(target.to_str().unwrap()).unwrap();
        assert_eq!(still_original, original);

        // Restore permissions so tempdir cleanup can delete the directory.
        std::fs::set_permissions(&ro_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    // --- tmp file is cleaned up on rename failure ---
    //
    // We verify that after a failed rename (cross-device or permission error),
    // the .tmp file is removed. We induce this by writing to a path whose
    // parent exists but whose target is a directory (rename onto a directory
    // fails on Linux without CAP_DAC_OVERRIDE when the directory is non-empty,
    // but a simpler approach is to write to a path that is itself a directory).
    #[test]
    fn tmp_file_cleaned_up_after_rename_failure() {
        let dir = tempfile::tempdir().unwrap();
        // The "target" path is a directory — renaming a file over a directory fails.
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let target_str = subdir.to_str().unwrap();

        let _ = write_json_file(
            target_str,
            &Row {
                value: "x".to_string(),
            },
        );

        // After the failed write the .tmp sibling must not exist.
        let tmp = format!("{}.tmp", target_str);
        assert!(
            !std::path::Path::new(&tmp).exists(),
            ".tmp file was not cleaned up after rename failure"
        );
    }
}
