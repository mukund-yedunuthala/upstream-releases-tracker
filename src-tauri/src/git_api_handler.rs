use reqwest::Client;
use std::sync::OnceLock;
use tracker_libs::{Config, RepoData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitHandler;

fn http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("Upstream-Release-Tracker")
            .build()
            .expect("Failed to build HTTP client")
    })
}

fn parse_url(url: &str) -> Result<(String, String), String> {
    let url = url.trim_end_matches('/');
    let parts: Vec<&str> = url.splitn(6, '/').collect();

    // Minimum: ["https:", "", "host", "owner", "repo"]
    if parts.len() < 5 {
        return Err(format!("URL does not look like a valid repo URL: {}", url));
    }

    let owner = parts[3].to_string();
    let repo  = parts[4].to_string();

    if owner.is_empty() || repo.is_empty() {
        return Err(format!("Could not extract owner/repo from URL: {}", url));
    }

    Ok((owner, repo))
}

async fn api_call(
    config: &Config,
    owner: &str,
    repo: &str,
    host: &str,
) -> Result<serde_json::Value, String> {
    match host {
        "github.com" => github_api_call(config, owner, repo).await,

        "gitlab.com" => {
            // TODO(gitlab): Implement GitLab releases API.
            // Endpoint: GET /api/v4/projects/{owner}%2F{repo}/releases
            // Auth header: "PRIVATE-TOKEN: <config.gitlab_token>"
            // Reference: https://docs.gitlab.com/ee/api/releases/
            Err("GitLab support is not yet implemented".to_string())
        }

        // _ if host == config.gitea_endpoint.trim_start_matches("https://").trim_end_matches('/') => {
        //     // TODO(gitea/forgejo): Implement Gitea/Forgejo releases API.
        //     // Endpoint: GET /api/v1/repos/{owner}/{repo}/releases/latest
        //     // Auth header: "Authorization: token <config.gitea_token>"
        //     // Forgejo uses the same API surface as Gitea.
        //     // Reference: https://gitea.io/api/swagger
        //     Err("Gitea/Forgejo support is not yet implemented".to_string())
        // }

        _ => Err(format!("Unsupported host '{}'", host)),
    }
}

async fn github_api_call(
    config: &Config,
    owner: &str,
    repo: &str,
) -> Result<serde_json::Value, String> {
    let api_url = format!(
        "{}{}/{}/releases/latest",
        config.github_endpoint, owner, repo
    );

    let mut request = http_client().get(&api_url);

    if !config.github_api_key.is_empty() {
        request = request.header(
            "Authorization",
            format!("Bearer {}", config.github_api_key),
        );
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "GitHub API returned error {}: {} for {}/{}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown"),
            owner,
            repo
        ));
    }

    response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse API response as JSON: {}", e))
}

impl GitHandler {
    pub async fn post_request(
            &self,
            config: &Config,
            url: String,
            host: String,
        ) -> Result<RepoData, String> {
            let (owner, repo_name) = parse_url(&url)?;
            let json = api_call(config, &owner, &repo_name, &host).await?;

            Ok(RepoData {
                owner,
                repo_name,
                host,
                latest_release: json["tag_name"].as_str().unwrap_or("").to_string(),
                system_version: String::new(),
            })
        }

    pub async fn refresh_repo(
        &self,
        config: &Config,
        old_repo: &RepoData,
    ) -> Result<RepoData, String> {
        let json = api_call(config, &old_repo.owner, &old_repo.repo_name, &old_repo.host).await?;

        Ok(RepoData {
            owner: old_repo.owner.clone(),
            repo_name: old_repo.repo_name.clone(),
            host: old_repo.host.clone(),
            latest_release: json["tag_name"].as_str().unwrap_or("").to_string(),
            system_version: old_repo.system_version.clone(),
        })
    }
}
