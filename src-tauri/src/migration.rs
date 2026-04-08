use crate::json_handler::JSONHandler;
use serde_json::Value;
use std::path::Path;

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

pub fn run_config_migrations(config_path: &str) {
    if !Path::new(config_path).exists() {
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

                // TODO(gitlab): Migration 2: add "gitlab_token" if absent.
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
