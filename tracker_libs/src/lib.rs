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

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct RepoData {
    pub owner: String,
    pub repo_name: String,
    pub latest_release: String,
    pub system_version: String,
    pub notes: String,
}
