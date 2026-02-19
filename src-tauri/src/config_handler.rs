use crate::json_handler::JSONHandler;
use std::fs;
use std::path::Path;
use tracker_libs::Config;

pub struct ConfigHandler {}

#[allow(dead_code)]
impl ConfigHandler {
    fn config_path() -> Result<String, String> {
        let config_dir = dirs::config_local_dir()
            .ok_or("Failed to locate local config directory")?;
        let path = config_dir.join("upstream-releases-tracker/config.json");
        Ok(path.to_str()
            .ok_or("Config path contains invalid UTF-8")?
            .to_string())
    }

    /// Reads config from disk. If the file does not exist, writes a default
    /// template and returns it so the user knows what to fill in.
    pub fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
        let filename = Self::config_path()?;
        let path = Path::new(&filename);

        if !path.exists() {
            // Create parent dirs and write a default template on first launch.
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let default_config = Config::new();
            JSONHandler::write_json_file::<Config>(filename.clone().as_str(), &default_config)?;
            eprintln!(
                "No config file found. A default template has been written to: {}. \
                 Please fill in your API token.",
                filename
            );
            return Ok(default_config);
        }

        JSONHandler::read_from_json::<Config>(filename.as_str())
    }

    pub fn write_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        let filename = Self::config_path()?;
        if let Some(parent) = Path::new(&filename).parent() {
            fs::create_dir_all(parent)?;
        }
        JSONHandler::write_json_file::<Config>(filename.as_str(), config)
    }
}
