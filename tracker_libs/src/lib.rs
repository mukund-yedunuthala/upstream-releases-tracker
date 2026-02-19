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
            github_endpoint: String::from("https://api.github.com/repos/"),
            gitlab_api_key: String::from(""),
            gitlab_endpoint: String::from("https://gitlab.com/api/v4/projects/"),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct RepoData {
    pub owner: String,
    pub repo_name: String,
    pub latest_release: String,
    pub system_version: String,
    pub host: String,
}

impl RepoData {
    pub fn new() -> RepoData {
        RepoData {
            owner: String::from(""),
            repo_name: String::from(""),
            latest_release: String::from(""),
            system_version: String::from(""),
            host: String::from(""),
        }
    }
}

#[allow(dead_code)]
pub enum RepoHost {
    GitHub,
    GitLab,
    /// Gitea or a Forgejo instance — identical API surface.
    ForgejoCompatible,
    Unknown,
}
