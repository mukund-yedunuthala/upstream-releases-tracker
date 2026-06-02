use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum ForgeKind {
    GitHub,
    GitLab,
    ForgejoCompatible, // Covers both Gitea and Forgejo — identical API surface
}

/// One entry in the Forgejo/Gitea trusted-host list.
///
/// `host` is the bare hostname (e.g. `"codeberg.org"` or `"forgejo.local:3000"`).
/// `token_ref` is the vault key whose secret holds the PAT for that host
/// (convention: `"forgejo_token:{host}"`). An empty `token_ref` means no
/// token is configured for that host — unauthenticated requests are sent.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct ForgejoHost {
    pub host: String,
    pub token_ref: String,
}

impl Default for ForgeKind {
    fn default() -> Self {
        ForgeKind::GitHub
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Config {
    pub github_api_key: String,
    pub github_endpoint: String,
    pub gitlab_api_key: String,
    pub gitlab_endpoint: String,
    /// Runtime-only: populated from the Stronghold vault at startup.
    /// Each entry holds the decrypted PAT for the corresponding host in
    /// `forgejo_trusted_hosts`. Indexed by position — same order as the list.
    /// Never written to config.json — skipped by serde entirely.
    #[serde(skip)]
    pub forgejo_tokens: Vec<String>,
    /// Allowlist of Forgejo/Gitea trusted hosts, each paired with the vault
    /// key name for its PAT. Persisted to config.json; tokens live in vault.
    pub forgejo_trusted_hosts: Vec<ForgejoHost>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            github_api_key: String::from(""),
            github_endpoint: String::from("https://api.github.com/repos/"),
            gitlab_api_key: String::from(""),
            gitlab_endpoint: String::from("https://gitlab.com/api/v4/projects/"),
            forgejo_tokens: vec![String::new()],
            forgejo_trusted_hosts: vec![ForgejoHost {
                host: "codeberg.org".to_string(),
                token_ref: "forgejo_token:codeberg.org".to_string(),
            }],
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
    #[serde(default)]
    pub release_notes: String,
}
