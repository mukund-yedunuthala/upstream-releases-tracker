use reqwest::Client;
use std::sync::OnceLock;
use tracker_libs::{Config, ForgeKind, RepoData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitHandler;

fn http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("Upstream-Release-Tracker")
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client")
    })
}

fn parse_url(url: &str) -> Result<(String, String), String> {
    if url.len() > 2048 {
        return Err("URL exceeds maximum allowed length of 2048 characters".to_string());
    }
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    if parsed.scheme() != "https" {
        return Err(format!(
            "Only HTTPS URLs are supported (got '{}')",
            parsed.scheme()
        ));
    }
    let mut segments = parsed
        .path_segments()
        .ok_or_else(|| "URL has no path segments".to_string())?
        .filter(|s| !s.is_empty());
    let owner = segments
        .next()
        .ok_or_else(|| "URL is missing owner segment".to_string())?
        .to_string();
    let repo = segments
        .next()
        .ok_or_else(|| "URL is missing repository segment".to_string())?
        .to_string();
    Ok((owner, repo))
}

async fn api_call(
    config: &Config,
    owner: &str,
    repo: &str,
    host_url: &str,
    host_kind: &ForgeKind,
) -> Result<serde_json::Value, String> {
    match host_kind {
        ForgeKind::GitHub => github_api_call(config, owner, repo).await,

        ForgeKind::ForgejoCompatible => forgejo_api_call(config, host_url, owner, repo).await,

        ForgeKind::GitLab => {
            // TODO(gitlab): Implement GitLab releases API.
            // Endpoint: GET https://gitlab.com/api/v4/projects/{owner}%2F{repo}/releases
            // Auth header: "PRIVATE-TOKEN: <config.gitlab_token>"
            // Note: owner/repo must be URL-encoded as a single slug: "owner%2Frepo"
            // Reference: https://docs.gitlab.com/ee/api/releases/
            Err("GitLab support is not yet implemented".to_string())
        }

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
        // Validate endpoint host before sending the token to prevent exfiltration
        // if github_endpoint is misconfigured to an attacker-controlled URL.
        let parsed = url::Url::parse(&api_url)
            .map_err(|e| format!("Invalid GitHub endpoint URL: {}", e))?;
        if parsed.host_str() != Some("api.github.com") {
            return Err(format!(
                "GitHub API token refused: endpoint host '{}' is not 'api.github.com'. \
                 Check github_endpoint in config.json.",
                parsed.host_str().unwrap_or("(none)")
            ));
        }
        request = request.header("Authorization", format!("Bearer {}", config.github_api_key));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "GitHub API returned {} ({}) for {}/{}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown"),
            owner,
            repo
        ));
    }

    response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse GitHub API response: {}", e))
}

async fn forgejo_api_call(
    config: &Config,
    host_url: &str,
    owner: &str,
    repo: &str,
) -> Result<serde_json::Value, String> {
    // Reject hosts not on the user-managed allowlist to prevent token exfiltration.
    // To add a new Forgejo host, add its hostname to forgejo_trusted_hosts in config.json.
    if !config.forgejo_token.is_empty()
        && !config
            .forgejo_trusted_hosts
            .iter()
            .any(|h| h.eq_ignore_ascii_case(host_url))
    {
        return Err(format!(
            "Host '{}' is not in forgejo_trusted_hosts. Add it to config.json to allow sending your token there.",
            host_url
        ));
    }

    // Forgejo and Gitea share the same API surface.
    // Endpoint: GET https://{host}/api/v1/repos/{owner}/{repo}/releases/latest
    let api_url = format!(
        "https://{}/api/v1/repos/{}/{}/releases/latest",
        host_url, owner, repo
    );

    let mut request = http_client().get(&api_url);

    if !config.forgejo_token.is_empty() {
        request = request.header("Authorization", format!("token {}", config.forgejo_token));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("HTTP request to '{}' failed: {}", host_url, e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Forgejo API at '{}' returned {} ({}) for {}/{}",
            host_url,
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown"),
            owner,
            repo
        ));
    }

    response.json::<serde_json::Value>().await.map_err(|e| {
        format!(
            "Failed to parse Forgejo API response from '{}': {}",
            host_url, e
        )
    })
}

impl GitHandler {
    pub async fn post_request(
        &self,
        config: &Config,
        url: String,
        host_url: String,
        host_kind: ForgeKind,
    ) -> Result<RepoData, String> {
        let (owner, repo_name) = parse_url(&url)?;
        let json = api_call(config, &owner, &repo_name, &host_url, &host_kind).await?;
        let latest_release = json["tag_name"]
            .as_str()
            .ok_or_else(|| "API response missing tag_name — repo may have no releases".to_string())?
            .to_string();

        Ok(RepoData {
            owner,
            repo_name,
            host_url,
            host_kind,
            latest_release,
            system_version: String::new(),
        })
    }

    pub async fn refresh_repo(
        &self,
        config: &Config,
        old_repo: &RepoData,
    ) -> Result<RepoData, String> {
        let json = api_call(
            config,
            &old_repo.owner,
            &old_repo.repo_name,
            &old_repo.host_url,
            &old_repo.host_kind,
        )
        .await?;
        let latest_release = json["tag_name"]
            .as_str()
            .ok_or_else(|| "API response missing tag_name — repo may have no releases".to_string())?
            .to_string();

        Ok(RepoData {
            owner: old_repo.owner.clone(),
            repo_name: old_repo.repo_name.clone(),
            host_url: old_repo.host_url.clone(),
            host_kind: old_repo.host_kind.clone(),
            latest_release,
            system_version: old_repo.system_version.clone(),
        })
    }
}
