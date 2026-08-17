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
                            obj.insert("release_notes".to_string(), Value::String(String::new()));
                            dirty = true;
                        }

                        // Migration 7: add "latest_release_timestamp" if absent.
                        if !obj.contains_key("latest_release_timestamp") {
                            obj.insert(
                                "latest_release_timestamp".to_string(),
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
                // in the OS keyring — config.json should only carry
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

                // Migration 6: convert forgejo_trusted_hosts from Vec<String>
                // (old format) to Vec<{host, token_ref}> (N2 per-host tokens).
                // Also removes the now-unused top-level forgejo_token field.
                //
                // Old format: ["codeberg.org", "gitea.example.com"]
                // New format: [
                //   {"host":"codeberg.org","token_ref":"forgejo_token:codeberg.org"},
                //   {"host":"gitea.example.com","token_ref":"forgejo_token:gitea.example.com"}
                // ]
                //
                // The existing single Forgejo token secret is NOT migrated
                // automatically — the user must re-enter tokens per host in Settings.
                // The old top-level forgejo_token key is removed from config.json
                // (it was already blanked by migration 5, so no secret is lost).
                if let Some(hosts_val) = obj.get("forgejo_trusted_hosts").cloned() {
                    let needs_upgrade = hosts_val
                        .as_array()
                        .map(|arr| {
                            // Needs upgrade if any element is a plain string.
                            arr.iter().any(|v| v.is_string())
                        })
                        .unwrap_or(false);

                    if needs_upgrade {
                        let new_hosts: Vec<Value> = hosts_val
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|host| {
                                serde_json::json!({
                                    "host": host,
                                    "token_ref": format!("forgejo_token:{}", host),
                                })
                            })
                            .collect();
                        obj.insert("forgejo_trusted_hosts".to_string(), Value::Array(new_hosts));
                        dirty = true;
                    }
                }

                // Migration 7: add "theme_mode" if absent.
                if !obj.contains_key("theme_mode") {
                    obj.insert(
                        "theme_mode".to_string(),
                        Value::String("system".to_string()),
                    );
                    dirty = true;
                }

                // Remove the now-redundant top-level forgejo_token field.
                // Config::new() no longer includes it; keep removing it so old
                // config.json files that still carry the blank field are cleaned up.
                if obj.remove("forgejo_token").is_some() {
                    dirty = true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;

    fn write_json(file: &NamedTempFile, value: &serde_json::Value) {
        let path = file.path().to_str().unwrap();
        crate::json_handler::write_json_file::<serde_json::Value>(path, value).unwrap();
    }

    fn read_json(file: &NamedTempFile) -> serde_json::Value {
        let path = file.path().to_str().unwrap();
        crate::json_handler::read_from_json::<serde_json::Value>(path).unwrap()
    }

    fn run_repos(file: &NamedTempFile) {
        run_migrations(file.path().to_str().unwrap());
    }

    fn run_config(file: &NamedTempFile) {
        run_config_migrations(file.path().to_str().unwrap());
    }

    // --- Data migrations ---

    #[test]
    fn data_m1_removes_notes_field() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "notes": "old", "owner": "a", "repo_name": "b", "host_url": "github.com", "host_kind": "GitHub", "latest_release": "v1", "system_version": "", "release_notes": "" } }),
        );
        run_repos(&file);
        let out = read_json(&file);
        assert!(out["https://github.com/a/b"].get("notes").is_none());
    }

    #[test]
    fn data_m2_renames_host_to_host_url() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "host": "github.com", "owner": "a", "repo_name": "b", "host_kind": "GitHub", "latest_release": "v1", "system_version": "", "release_notes": "" } }),
        );
        run_repos(&file);
        let out = read_json(&file);
        let repo = &out["https://github.com/a/b"];
        assert_eq!(repo["host_url"], json!("github.com"));
        assert!(repo.get("host").is_none());
    }

    #[test]
    fn data_m3_adds_default_host_url() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "owner": "a", "repo_name": "b", "host_kind": "GitHub", "latest_release": "v1", "system_version": "", "release_notes": "" } }),
        );
        run_repos(&file);
        let out = read_json(&file);
        assert_eq!(
            out["https://github.com/a/b"]["host_url"],
            json!("github.com")
        );
    }

    #[test]
    fn data_m4_adds_default_host_kind() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "owner": "a", "repo_name": "b", "host_url": "github.com", "latest_release": "v1", "system_version": "", "release_notes": "" } }),
        );
        run_repos(&file);
        let out = read_json(&file);
        assert_eq!(out["https://github.com/a/b"]["host_kind"], json!("GitHub"));
    }

    #[test]
    fn data_m5_coerces_unknown_host_kind_to_github() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "owner": "a", "repo_name": "b", "host_url": "github.com", "host_kind": "Unknown", "latest_release": "v1", "system_version": "", "release_notes": "" } }),
        );
        run_repos(&file);
        let out = read_json(&file);
        assert_eq!(out["https://github.com/a/b"]["host_kind"], json!("GitHub"));
    }

    #[test]
    fn data_m6_adds_release_notes_if_absent() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "owner": "a", "repo_name": "b", "host_url": "github.com", "host_kind": "GitHub", "latest_release": "v1", "system_version": "" } }),
        );
        run_repos(&file);
        let out = read_json(&file);
        assert_eq!(out["https://github.com/a/b"]["release_notes"], json!(""));
    }

    #[test]
    fn data_m7_adds_latest_release_timestamp_if_absent() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "owner": "a", "repo_name": "b", "host_url": "github.com", "host_kind": "GitHub", "latest_release": "v1", "system_version": "" } }),
        );
        run_repos(&file);
        let out = read_json(&file);
        assert_eq!(
            out["https://github.com/a/b"]["latest_release_timestamp"],
            json!("")
        );
    }

    #[test]
    fn data_m7_idempotent() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "owner": "a", "repo_name": "b", "host_url": "github.com", "host_kind": "GitHub", "latest_release": "v1", "system_version": "", "latest_release_timestamp": "2024-01-15T10:30:00Z" } }),
        );
        run_repos(&file);
        let after_one = read_json(&file);
        run_repos(&file);
        let after_two = read_json(&file);
        assert_eq!(after_one, after_two);
    }

    #[test]
    fn data_idempotent() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "https://github.com/a/b": { "notes": "x", "host": "github.com", "owner": "a", "repo_name": "b", "host_kind": "Unknown", "latest_release": "v1", "system_version": "", "latest_release_timestamp": "" } }),
        );
        run_repos(&file);
        let after_one = read_json(&file);
        run_repos(&file);
        let after_two = read_json(&file);
        assert_eq!(after_one, after_two);
    }

    #[test]
    fn data_missing_file_no_crash() {
        // Should return immediately without panicking.
        run_migrations("/tmp/__upstream_tracker_nonexistent_test_file_12345.json");
    }

    // --- Config migrations ---

    #[test]
    fn config_m1_adds_forgejo_token_then_m6_removes_it() {
        // Migration 1 adds forgejo_token; migration 6 then removes the field.
        // After both run, the key must be absent (not "" — it's gone entirely).
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "github_endpoint": "https://api.github.com/repos/" }),
        );
        run_config(&file);
        let out = read_json(&file);
        assert!(
            out.get("forgejo_token").is_none(),
            "forgejo_token should be removed by migration 6, got: {:?}",
            out.get("forgejo_token")
        );
    }

    #[test]
    fn config_m2_adds_forgejo_trusted_hosts_as_objects() {
        // Migration 2 adds the default codeberg.org host; migration 6 upgrades
        // the plain string to the object format used since N2.
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "github_endpoint": "https://api.github.com/repos/" }),
        );
        run_config(&file);
        let out = read_json(&file);
        assert_eq!(
            out["forgejo_trusted_hosts"],
            json!([{"host": "codeberg.org", "token_ref": "forgejo_token:codeberg.org"}])
        );
    }

    #[test]
    fn config_m6_upgrades_string_host_list_to_objects() {
        // Simulates upgrading from a pre-N2 config that has Vec<String> hosts.
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({
                "github_endpoint": "https://api.github.com/repos/",
                "forgejo_trusted_hosts": ["codeberg.org", "gitea.example.com"],
            }),
        );
        run_config(&file);
        let out = read_json(&file);
        assert_eq!(
            out["forgejo_trusted_hosts"],
            json!([
                {"host": "codeberg.org", "token_ref": "forgejo_token:codeberg.org"},
                {"host": "gitea.example.com", "token_ref": "forgejo_token:gitea.example.com"},
            ])
        );
    }

    #[test]
    fn config_m6_object_host_list_unchanged() {
        // If hosts are already in object format, migration 6 must not re-wrap them.
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({
                "github_endpoint": "https://api.github.com/repos/",
                "forgejo_trusted_hosts": [
                    {"host": "codeberg.org", "token_ref": "forgejo_token:codeberg.org"},
                ],
            }),
        );
        run_config(&file);
        let out = read_json(&file);
        assert_eq!(
            out["forgejo_trusted_hosts"],
            json!([{"host": "codeberg.org", "token_ref": "forgejo_token:codeberg.org"}])
        );
    }

    #[test]
    fn config_m3_adds_gitlab_api_key() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "github_endpoint": "https://api.github.com/repos/" }),
        );
        run_config(&file);
        let out = read_json(&file);
        assert_eq!(out["gitlab_api_key"], json!(""));
    }

    #[test]
    fn config_m4_adds_gitlab_endpoint() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "github_endpoint": "https://api.github.com/repos/" }),
        );
        run_config(&file);
        let out = read_json(&file);
        assert_eq!(
            out["gitlab_endpoint"],
            json!("https://gitlab.com/api/v4/projects/")
        );
    }

    #[test]
    fn config_m5_scrubs_plaintext_api_keys() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({
                "github_api_key": "ghp_secret",
                "gitlab_api_key": "glpat_secret",
                "forgejo_token": "fgt_secret"
            }),
        );
        run_config(&file);
        let out = read_json(&file);
        assert_eq!(out["github_api_key"], json!(""));
        assert_eq!(out["gitlab_api_key"], json!(""));
        // Migration 5 blanks forgejo_token; migration 6 then removes the field.
        assert!(
            out.get("forgejo_token").is_none(),
            "forgejo_token should be removed by migration 6"
        );
    }

    #[test]
    fn config_m5_skips_already_empty_keys() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({
                "github_api_key": "",
                "gitlab_api_key": "",
                "forgejo_token": ""
            }),
        );
        // Write once so we have a baseline mtime, then run migrations.
        // The file should not be rewritten (dirty stays false).
        run_config(&file);
        let out = read_json(&file);
        assert_eq!(out["github_api_key"], json!(""));
    }

    #[test]
    fn config_idempotent() {
        let file = NamedTempFile::new().unwrap();
        write_json(&file, &json!({ "github_api_key": "secret" }));
        run_config(&file);
        let after_one = read_json(&file);
        run_config(&file);
        let after_two = read_json(&file);
        assert_eq!(after_one, after_two);
    }

    #[test]
    fn config_missing_file_no_crash() {
        run_config_migrations("/tmp/__upstream_tracker_nonexistent_config_12345.json");
    }

    #[test]
    fn config_m7_adds_theme_mode_if_absent() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "github_endpoint": "https://api.github.com/repos/" }),
        );
        run_config(&file);
        let out = read_json(&file);
        assert_eq!(out["theme_mode"], json!("system"));
    }

    #[test]
    fn config_m7_idempotent() {
        let file = NamedTempFile::new().unwrap();
        write_json(
            &file,
            &json!({ "github_endpoint": "https://api.github.com/repos/", "theme_mode": "dark" }),
        );
        run_config(&file);
        let after_one = read_json(&file);
        run_config(&file);
        let after_two = read_json(&file);
        assert_eq!(after_one, after_two);
        assert_eq!(after_two["theme_mode"], json!("dark"));
    }
}
