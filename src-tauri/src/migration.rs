use crate::json_handler;
use serde_json::Value;
use std::path::Path;

pub fn run_migrations(datafile_path: &str) {
    if !Path::new(datafile_path).exists() {
        return;
    }

    match json_handler::read_from_json::<Value>(datafile_path) {
        Ok(mut raw) => {
            let mut dirty = false;

            if let Some(repos) = raw.as_object_mut() {
                for (_url, repo) in repos.iter_mut() {
                    if let Some(obj) = repo.as_object_mut() {
                        if obj.remove("notes").is_some() {
                            dirty = true;
                        }
                        if let Some(host_val) = obj.remove("host") {
                            obj.insert("host_url".to_string(), host_val);
                            dirty = true;
                        }
                        if !obj.contains_key("host_url") {
                            obj.insert(
                                "host_url".to_string(),
                                Value::String("github.com".to_string()),
                            );
                            dirty = true;
                        }
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

                        // Migration 6: add "release_notes" if absent.
                        if !obj.contains_key("release_notes") {
                            obj.insert(
                                "release_notes".to_string(),
                                Value::String(String::new()),
                            );
                            dirty = true;
                        }
                    }
                }
            }

            if dirty {
                match json_handler::write_json_file::<Value>(datafile_path, &raw) {
                    Ok(_) => log::info!("repos.json migration completed successfully."),
                    Err(e) => log::error!("Failed to write migrated repos.json: {}", e),
                }
            }
        }
        Err(e) => log::warn!("repos.json migration skipped: {}", e),
    }
}

pub fn run_config_migrations(config_path: &str) {
    if !Path::new(config_path).exists() {
        return;
    }

    match json_handler::read_from_json::<Value>(config_path) {
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

                // Migration 3: add "gitlab_api_key" if absent.
                if !obj.contains_key("gitlab_api_key") {
                    obj.insert("gitlab_api_key".to_string(), Value::String(String::new()));
                    dirty = true;
                }

                // Migration 4: add "gitlab_endpoint" if absent.
                if !obj.contains_key("gitlab_endpoint") {
                    obj.insert(
                        "gitlab_endpoint".to_string(),
                        Value::String("https://gitlab.com/api/v4/projects/".to_string()),
                    );
                    dirty = true;
                }

                // Migration 5: scrub plaintext API key fields. Secrets now live
                // in the Stronghold vault — config.json should only carry
                // endpoint metadata. We blank the fields (rather than removing
                // them) so the Config struct still deserializes on read.
                for key in ["github_api_key", "gitlab_api_key", "forgejo_token"] {
                    if let Some(existing) = obj.get(key) {
                        let is_nonempty = existing.as_str().map(|s| !s.is_empty()).unwrap_or(false);
                        if is_nonempty {
                            obj.insert(key.to_string(), Value::String(String::new()));
                            dirty = true;
                        }
                    }
                }
            }

            if dirty {
                match json_handler::write_json_file::<Value>(config_path, &raw) {
                    Ok(_) => log::info!("config.json migration completed successfully."),
                    Err(e) => log::error!("Failed to write migrated config.json: {}", e),
                }
            }
        }
        Err(e) => log::warn!("config.json migration skipped: {}", e),
    }
}
