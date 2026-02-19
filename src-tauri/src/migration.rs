use crate::json_handler::JSONHandler;
use serde_json::Value;
use std::path::Path;

/// Runs all one-time data migrations against repos.json.
/// Safe to call on every startup — each migration is idempotent.
pub fn run_migrations(datafile_path: &str) {
    if !Path::new(datafile_path).exists() {
        // Nothing to migrate on a fresh install.
        return;
    }

    match JSONHandler::read_from_json::<Value>(datafile_path) {
        Ok(mut raw) => {
            let mut dirty = false;

            if let Some(repos) = raw.as_object_mut() {
                for (_url, repo) in repos.iter_mut() {
                    if let Some(obj) = repo.as_object_mut() {
                        // Migration 1: drop "notes" if present.
                        if obj.remove("notes").is_some() {
                            dirty = true;
                        }

                        // Migration 2: add "host" defaulting to "github.com"
                        // if not already present.
                        if !obj.contains_key("host") {
                            obj.insert(
                                "host".to_string(),
                                Value::String("github.com".to_string()),
                            );
                            dirty = true;
                        }
                    }
                }
            }

            if dirty {
                match JSONHandler::write_json_file::<Value>(datafile_path, &raw) {
                    Ok(_) => eprintln!("Data migration completed successfully."),
                    Err(e) => eprintln!("Failed to write migrated data: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Migration skipped: could not read data file: {}", e),
    }
}
