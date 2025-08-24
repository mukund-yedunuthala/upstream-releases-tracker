use reqwest::blocking::Client;
use tracker_libs::{Config, RepoData};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitHandler;

fn parse_url(url: String) -> Result<(String, String), String> {
    let repo = url
        .trim_end_matches('/')
        .split('/')
        .rev()
        .take(2)
        .collect::<Vec<_>>();
    if repo.len() < 2 {
        return Err("Invalid GitHub URL".into());
    } else {
        return Ok((repo[1].to_string(), repo[0].to_string()));
    }
}

fn api_call(config: &Config, owner: String, repo: String) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let api_url = format!(
        "{}{}/{}/releases/latest",
        config.github_endpoint, owner, repo
    );
    let response = client
        .get(&api_url)
        .header("User-Agent", "Upstream-Release-Tracker")
        .send()
        .map_err(|_| "Failed to fetch release data".to_string())?;
    let resp_as_json: serde_json::Value = response
        .json()
        .map_err(|_| "Invalid response".to_string())?;

    Ok(resp_as_json)
}

impl GitHandler {
    pub fn post_request(&self, config: &Config, url: String) -> Result<RepoData, String> {
        match parse_url(url) {
            Ok((r_owner, r_repo)) => match api_call(config, r_owner.clone(), r_repo.clone()) {
                Ok(resp_as_json) => {
                    let new_repo: RepoData = RepoData {
                        owner: r_owner,
                        repo_name: r_repo,
                        latest_release: resp_as_json["tag_name"].as_str().unwrap_or("").to_string(),
                        system_version: resp_as_json["tag_name"].as_str().unwrap_or("").to_string(),
                        notes: resp_as_json["body"].as_str().unwrap_or("").to_string(),
                    };
                    Ok(new_repo)
                }
                Err(e) => Err(format!("Failed to call api with error: {}", e).to_string()),
            },
            Err(e) => Err(format!("Failed to parse source url with error: {}", e).to_string()),
        }
    }
    pub fn refresh_repo(&self, config: &Config, old_repo: &RepoData) -> Result<RepoData, String> {
        match api_call(config, old_repo.owner.clone(), old_repo.repo_name.clone()) {
            Ok(resp_as_json) => {
                let new_repo: RepoData = RepoData {
                    owner: old_repo.owner.clone(),
                    repo_name: old_repo.repo_name.clone(),
                    latest_release: resp_as_json["tag_name"].as_str().unwrap_or("").to_string(),
                    system_version: old_repo.system_version.clone(),
                    notes: resp_as_json["body"].as_str().unwrap_or("").to_string(),
                };
                Ok(new_repo)
            }
            Err(e) => Err(format!("Failed to refresh repo with error {}", e)).into(),
        }
    }
}
