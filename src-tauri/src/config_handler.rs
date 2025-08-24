use crate::json_handler::JSONHandler;
pub struct ConfigHandler {}
use tracker_libs::Config;

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
