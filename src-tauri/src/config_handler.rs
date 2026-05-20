use crate::json_handler;
use std::fs;
use std::path::Path;
use tracker_libs::Config;

pub fn config_path_string() -> Result<String, String> {
    let config_dir =
        dirs::config_local_dir().ok_or("Failed to locate local config directory")?;
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
