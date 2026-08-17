use crate::Config;
use crate::json_handler;
use std::fs;
use std::path::Path;

pub fn config_path_string() -> Result<String, String> {
    let config_dir = dirs::config_local_dir().ok_or("Failed to locate local config directory")?;
    let path = config_dir.join("upstream-releases-tracker/config.json");
    path.to_str()
        .ok_or("Config path contains invalid UTF-8".to_string())
        .map(|s| s.to_string())
}

pub fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
    let filename = config_path_string()?;
    let path = Path::new(&filename);

    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let default_config = Config::default();
        json_handler::write_json_file::<Config>(&filename, &default_config)?;
        eprintln!(
            "No config file found. A default template has been written to: {}. \
             Please fill in your API tokens.",
            filename
        );
        return Ok(default_config);
    }

    json_handler::read_from_json::<Config>(&filename)
}

pub fn write_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let filename = config_path_string()?;
    if let Some(parent) = Path::new(&filename).parent() {
        fs::create_dir_all(parent)?;
    }
    json_handler::write_json_file::<Config>(&filename, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ForgejoHost;
    use serde_json::json;
    use tempfile::NamedTempFile;

    fn write_and_read(config: &Config) -> Config {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        json_handler::write_json_file::<Config>(path, config).unwrap();
        json_handler::read_from_json::<Config>(path).unwrap()
    }

    #[test]
    fn round_trip_default() {
        let config = Config::default();
        let out = write_and_read(&config);
        // forgejo_tokens is #[serde(skip)] — it deserialises as empty Vec, so
        // compare only the fields that are actually persisted to disk.
        assert_eq!(out.github_endpoint, config.github_endpoint);
        assert_eq!(out.gitlab_endpoint, config.gitlab_endpoint);
        assert_eq!(out.forgejo_trusted_hosts, config.forgejo_trusted_hosts);
        assert_eq!(out.github_api_key, config.github_api_key);
        assert_eq!(out.gitlab_api_key, config.gitlab_api_key);
        assert_eq!(out.theme_mode, config.theme_mode);
    }

    #[test]
    fn round_trip_with_endpoints() {
        let mut config = Config::new();
        config.github_endpoint = "https://github.example.com/api/".to_string();
        config.gitlab_endpoint = "https://gitlab.example.com/api/v4/projects/".to_string();
        let out = write_and_read(&config);
        assert_eq!(out.github_endpoint, config.github_endpoint);
        assert_eq!(out.gitlab_endpoint, config.gitlab_endpoint);
        assert_eq!(out.theme_mode, config.theme_mode);
    }

    #[test]
    fn old_config_without_theme_mode_deserializes_as_system() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        json_handler::write_json_file::<serde_json::Value>(
            path,
            &json!({
                "github_api_key": "",
                "github_endpoint": "https://api.github.com/repos/",
                "gitlab_api_key": "",
                "gitlab_endpoint": "https://gitlab.com/api/v4/projects/",
                "forgejo_trusted_hosts": [],
            }),
        )
        .unwrap();
        let out: Config = json_handler::read_from_json(path).unwrap();
        assert_eq!(out.theme_mode, "system");
    }

    #[test]
    fn forgejo_tokens_not_serialized() {
        let mut config = Config::new();
        config.forgejo_tokens = vec!["secret_token".to_string()];
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        json_handler::write_json_file::<Config>(path, &config).unwrap();
        let raw: serde_json::Value = json_handler::read_from_json(path).unwrap();
        assert!(
            raw.get("forgejo_tokens").is_none(),
            "forgejo_tokens must not appear in config.json (it is #[serde(skip)])"
        );
    }

    #[test]
    fn forgejo_trusted_hosts_round_trip() {
        let mut config = Config::new();
        config.forgejo_trusted_hosts = vec![
            ForgejoHost {
                host: "codeberg.org".to_string(),
                token_ref: "forgejo_token:codeberg.org".to_string(),
            },
            ForgejoHost {
                host: "forgejo.example.com".to_string(),
                token_ref: "forgejo_token:forgejo.example.com".to_string(),
            },
        ];
        let out = write_and_read(&config);
        assert_eq!(out.forgejo_trusted_hosts.len(), 2);
        assert_eq!(out.forgejo_trusted_hosts[0].host, "codeberg.org");
        assert_eq!(
            out.forgejo_trusted_hosts[1].token_ref,
            "forgejo_token:forgejo.example.com"
        );
    }
}
