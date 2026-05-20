use crate::json_handler::JSONHandler;
use serde_json::Value;
use std::path::Path;

/// Migrates repos.json schema. Safe to call every startup — idempotent.
pub fn run_migrations(datafile_path: &str) {
    if !Path::new(datafile_path).exists() {
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

                        // Migration 2: rename "host" → "host_url".
                        if let Some(host_val) = obj.remove("host") {
                            obj.insert("host_url".to_string(), host_val);
                            dirty = true;
                        }

                        // Migration 3: add "host_url" defaulting to "github.com"
                        // if still absent after migration 2.
                        if !obj.contains_key("host_url") {
                            obj.insert(
                                "host_url".to_string(),
                                Value::String("github.com".to_string()),
                            );
                            dirty = true;
                        }

                        // Migration 4: add "host_kind" defaulting to "GitHub"
                        // for all pre-existing entries.
                        if !obj.contains_key("host_kind") {
                            obj.insert(
                                "host_kind".to_string(),
                                Value::String("GitHub".to_string()),
                            );
                            dirty = true;
                        }

                        // Migration 5: coerce the removed ForgeKind::Unknown
                        // variant to "GitHub" so old data files still deserialize.
                        if obj.get("host_kind").and_then(|v| v.as_str()) == Some("Unknown") {
                            obj.insert(
                                "host_kind".to_string(),
                                Value::String("GitHub".to_string()),
                            );
                            dirty = true;
                        }
                    }
                }
            }

            if dirty {
                match JSONHandler::write_json_file::<Value>(datafile_path, &raw) {
                    Ok(_) => eprintln!("repos.json migration completed successfully."),
                    Err(e) => eprintln!("Failed to write migrated repos.json: {}", e),
                }
            }
        }
        Err(e) => eprintln!("repos.json migration skipped: {}", e),
    }
}

/// Migrates config.json schema. Safe to call every startup — idempotent.
pub fn run_config_migrations(config_path: &str) {
    if !Path::new(config_path).exists() {
        // Config doesn't exist yet — ConfigHandler::read_config will create
        // a fresh template with all current fields, no migration needed.
        return;
    }

    match JSONHandler::read_from_json::<Value>(config_path) {
        Ok(mut raw) => {
            let mut dirty = false;

            if let Some(obj) = raw.as_object_mut() {
                // Migration 1: add "forgejo_token" if absent.
                if !obj.contains_key("forgejo_token") {
                    obj.insert("forgejo_token".to_string(), Value::String(String::new()));
                    dirty = true;
                }

                // Migration 2: add "forgejo_trusted_hosts" if absent.
                // Default: ["codeberg.org"] so existing Codeberg users keep working.
                if !obj.contains_key("forgejo_trusted_hosts") {
                    obj.insert(
                        "forgejo_trusted_hosts".to_string(),
                        serde_json::json!(["codeberg.org"]),
                    );
                    dirty = true;
                }

                // TODO(gitlab): Migration: add "gitlab_token" if absent.
                // if !obj.contains_key("gitlab_token") {
                //     obj.insert("gitlab_token".to_string(), Value::String(String::new()));
                //     dirty = true;
                // }
            }

            if dirty {
                match JSONHandler::write_json_file::<Value>(config_path, &raw) {
                    Ok(_) => eprintln!("config.json migration completed successfully."),
                    Err(e) => eprintln!("Failed to write migrated config.json: {}", e),
                }
            }
        }
        Err(e) => eprintln!("config.json migration skipped: {}", e),
    }
}
