use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum ForgeKind {
    GitHub,
    GitLab,
    ForgejoCompatible, // Covers both Gitea and Forgejo — identical API surface
}

impl Default for ForgeKind {
    fn default() -> Self {
        ForgeKind::GitHub
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Config {
    pub github_api_key: String,
    pub github_endpoint: String,
    pub gitlab_api_key: String,
    pub gitlab_endpoint: String,
    pub forgejo_token: String,
    /// Allowlist of Forgejo/Gitea hostnames the token may be sent to.
    /// The token is never transmitted to a host not on this list.
    pub forgejo_trusted_hosts: Vec<String>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            github_api_key: String::from(""),
            github_endpoint: String::from("https://api.github.com/repos/"),
            gitlab_api_key: String::from(""),
            gitlab_endpoint: String::from("https://gitlab.com/api/v4/projects/"),
            forgejo_token: String::new(),
            forgejo_trusted_hosts: vec!["codeberg.org".to_string()],
        }
    }
}

// The BTreeMap key (the full URL) duplicates host_url + owner + repo_name. This
// is intentional denormalization — O(log n) lookup by URL outweighs the minor
// drift risk for this dataset size.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct RepoData {
    pub owner: String,
    pub repo_name: String,
    pub host_url: String,     // actual domain, e.g. "codeberg.org", "github.com"
    pub host_kind: ForgeKind, // user-selected forge type
    pub latest_release: String,
    pub system_version: String,
}
