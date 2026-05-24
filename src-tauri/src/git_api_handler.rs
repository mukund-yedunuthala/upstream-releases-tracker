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

/// Parses a repo URL into (owner, repo) where owner may be a slash-joined
/// path for GitLab subgroups (e.g. "group/subgroup" from
/// https://gitlab.com/group/subgroup/project). For GitHub and Forgejo
/// URLs the owner is always a single segment.
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
    let segments: Vec<&str> = parsed
        .path_segments()
        .ok_or_else(|| "URL has no path segments".to_string())?
        .filter(|s| !s.is_empty())
        .collect();
    if segments.len() < 2 {
        return Err("URL must have at least owner and repository segments".to_string());
    }
    let repo = segments[segments.len() - 1].to_string();
    let owner = segments[..segments.len() - 1].join("/");
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

        ForgeKind::GitLab => gitlab_api_call(config, owner, repo).await,

    }
}

async fn gitlab_api_call(
    config: &Config,
    owner: &str,
    repo: &str,
) -> Result<serde_json::Value, String> {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    // GitLab requires the project path to be URL-encoded as a single slug.
    // The slash separator between namespace and project is the literal %2F;
    // owner and repo segments are each percent-encoded over NON_ALPHANUMERIC
    // so that dots, hyphens, and underscores are also encoded — a conservative
    // choice that prevents path-injection if validation is relaxed later (#50).
    let encode_segment = |s: &str| utf8_percent_encode(s, NON_ALPHANUMERIC).to_string();
    let project_slug = format!("{}%2F{}", encode_segment(owner), encode_segment(repo));
    let api_url = format!("{}{}/releases", config.gitlab_endpoint, project_slug);

    let parsed = url::Url::parse(&api_url)
        .map_err(|e| format!("Invalid GitLab endpoint URL: {}", e))?;
    if parsed.scheme() != "https" {
        return Err(format!(
            "GitLab endpoint must use HTTPS (got '{}'). Check gitlab_endpoint in settings.",
            parsed.scheme()
        ));
    }
    if parsed.host_str().is_none() {
        return Err("GitLab endpoint URL has no host. Check gitlab_endpoint in settings.".to_string());
    }

    let mut request = http_client().get(&api_url);

    if !config.gitlab_api_key.is_empty() {
        request = request.header("PRIVATE-TOKEN", &config.gitlab_api_key);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "GitLab API returned {} ({}) for {}/{}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown"),
            owner,
            repo
        ));
    }

    // GitLab returns an array of releases ordered newest-first. Return the
    // first element so callers can extract `tag_name` uniformly across forges.
    let releases: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitLab API response: {}", e))?;

    releases
        .get(0)
        .cloned()
        .ok_or_else(|| format!("No releases found for {}/{}", owner, repo))
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

fn extract_release_notes(json: &serde_json::Value, host_kind: &ForgeKind) -> String {
    let key = match host_kind {
        ForgeKind::GitLab => "description",
        _ => "body",
    };
    json[key].as_str().unwrap_or("").to_string()
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
        let release_notes = extract_release_notes(&json, &host_kind);

        Ok(RepoData {
            owner,
            repo_name,
            host_url,
            host_kind,
            latest_release,
            system_version: String::new(),
            release_notes,
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
        let release_notes = extract_release_notes(&json, &old_repo.host_kind);

        Ok(RepoData {
            owner: old_repo.owner.clone(),
            repo_name: old_repo.repo_name.clone(),
            host_url: old_repo.host_url.clone(),
            host_kind: old_repo.host_kind.clone(),
            latest_release,
            system_version: old_repo.system_version.clone(),
            release_notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_url;

    #[test]
    fn parse_url_github_flat() {
        assert_eq!(
            parse_url("https://github.com/torvalds/linux").unwrap(),
            ("torvalds".to_string(), "linux".to_string())
        );
    }

    #[test]
    fn parse_url_github_trailing_slash() {
        assert_eq!(
            parse_url("https://github.com/torvalds/linux/").unwrap(),
            ("torvalds".to_string(), "linux".to_string())
        );
    }

    #[test]
    fn parse_url_gitlab_subgroup() {
        assert_eq!(
            parse_url("https://gitlab.com/group/subgroup/project").unwrap(),
            ("group/subgroup".to_string(), "project".to_string())
        );
    }

    #[test]
    fn parse_url_gitlab_deep_subgroup() {
        assert_eq!(
            parse_url("https://gitlab.com/a/b/c/d").unwrap(),
            ("a/b/c".to_string(), "d".to_string())
        );
    }

    #[test]
    fn parse_url_rejects_http() {
        assert!(parse_url("http://github.com/owner/repo").is_err());
    }

    #[test]
    fn parse_url_rejects_single_segment() {
        assert!(parse_url("https://github.com/owner").is_err());
    }
}
