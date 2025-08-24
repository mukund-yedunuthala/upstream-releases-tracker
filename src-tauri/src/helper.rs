use crate::config_handler::ConfigHandler;
use regex::Regex;
use tracker_libs::Config;

pub fn read_config() -> Config {
    let mut config_struct = Config::new();
    match ConfigHandler::read_config() {
        Ok(config) => {
            config_struct = config;
        }
        Err(e) => eprintln!("Error reading configuration file: {}", e),
    }
    config_struct
}

pub fn is_valid_repo_url(url: &str) -> bool {
    // Regular expression pattern for GitHub repository URLs
    let github_pattern =
        Regex::new(r"^https://github\.com/[a-zA-Z0-9_-]+/[a-zA-Z0-9_-]+$").unwrap();

    // Regular expression pattern for GitLab repository URLs
    let gitlab_pattern =
        Regex::new(r"^https://gitlab\.com/[a-zA-Z0-9_-]+/[a-zA-Z0-9_-]+$").unwrap();

    // Check if the URL matches either GitHub or GitLab pattern
    github_pattern.is_match(url) || gitlab_pattern.is_match(url)
}
