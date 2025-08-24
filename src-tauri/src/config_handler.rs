use crate::json_handler::JSONHandler;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Config {
    pub github_api_key: String,
    pub github_endpoint: String,
    pub gitlab_api_key: String,
    pub gitlab_endpoint: String,
}

impl Config {
    pub fn new() -> Config {
        Config {
            github_api_key: String::from(""),
            github_endpoint: String::from(""),
            gitlab_api_key: String::from(""),
            gitlab_endpoint: String::from(""),
        }
    }
}

pub struct ConfigHandler {}

impl ConfigHandler {
    pub fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
        let configfile_pathbuf = dirs::config_local_dir()
            .unwrap()
            .join("upstream-releases-tracker/config.json");
        let filename: String = String::from(configfile_pathbuf.to_str().unwrap());
        match JSONHandler::read_from_json::<Config>(filename) {
            Ok(config) => Ok(config),
            e => e,
        }
    }
}
