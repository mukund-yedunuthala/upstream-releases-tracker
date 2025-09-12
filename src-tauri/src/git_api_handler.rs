use reqwest::Client;
use tracker_libs::{Config, RepoData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitHandler;

fn parse_url(url: String) -> Result<(String, String), String> {
    let repo: Vec<&str> = url.trim_end_matches('/').split('/').rev().take(2).collect();
    if repo.len() < 2 {
        Err("Invalid GitHub URL".into())
    } else {
        // repo[1] = owner, repo[0] = repo name (since .rev())
        let owner = repo[1].to_string();
        let repo_name = repo[0].to_string();
        Ok((owner, repo_name))
    }
}

// Now fully async and propagating errors via Result
async fn api_call(
    config: &Config,
    owner: String,
    repo: String,
) -> Result<serde_json::Value, String> {
    let client = Client::new();
    let api_url = format!(
        "{}{}/{}/releases/latest",
        config.github_endpoint, owner, repo
    );
    let response = client
        .get(&api_url)
        .header("User-Agent", "Upstream-Release-Tracker")
        .send()
        .await
        .map_err(|_| "Failed to fetch release data".to_string())?;
    let resp_as_json = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "Invalid response".to_string())?;
    Ok(resp_as_json)
}

// All public methods that call network APIs become async
impl GitHandler {
    pub async fn post_request(&self, config: &Config, url: String) -> Result<RepoData, String> {
        let (r_owner, r_repo) = parse_url(url)?;
        let resp_as_json = api_call(config, r_owner.clone(), r_repo.clone()).await?;
        let new_repo = RepoData {
            owner: r_owner,
            repo_name: r_repo,
            latest_release: resp_as_json["tag_name"].as_str().unwrap_or("").to_string(),
            system_version: resp_as_json["tag_name"].as_str().unwrap_or("").to_string(),
            notes: resp_as_json["body"].as_str().unwrap_or("").to_string(),
        };
        Ok(new_repo)
    }

    pub async fn refresh_repo(
        &self,
        config: &Config,
        old_repo: &RepoData,
    ) -> Result<RepoData, String> {
        let resp_as_json =
            api_call(config, old_repo.owner.clone(), old_repo.repo_name.clone()).await?;
        let new_repo = RepoData {
            owner: old_repo.owner.clone(),
            repo_name: old_repo.repo_name.clone(),
            latest_release: resp_as_json["tag_name"].as_str().unwrap_or("").to_string(),
            system_version: old_repo.system_version.clone(),
            notes: resp_as_json["body"].as_str().unwrap_or("").to_string(),
        };
        Ok(new_repo)
    }
}
