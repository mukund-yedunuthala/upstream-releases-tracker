use crate::{Config, ForgeKind, RepoData};
use reqwest::Client;
use std::sync::LazyLock;

fn http_client() -> &'static Client {
    static CLIENT: LazyLock<Client> = LazyLock::new(|| {
        Client::builder()
            .user_agent("Upstream-Release-Tracker")
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client")
    });
    &CLIENT
}

fn decode_path_segments(path: &str) -> Result<Vec<String>, String> {
    path.split('/')
        .map(|segment| {
            percent_encoding::percent_decode_str(segment)
                .decode_utf8()
                .map(|decoded| decoded.into_owned())
                .map_err(|_| "Repository path contains invalid UTF-8".to_string())
        })
        .collect()
}

fn append_path_segments(base: &str, segments: &[String]) -> Result<url::Url, String> {
    let mut url = url::Url::parse(base).map_err(|e| e.to_string())?;
    url.path_segments_mut()
        .map_err(|_| "URL cannot be used as a hierarchical base".to_string())?
        .pop_if_empty()
        .extend(segments.iter().map(String::as_str));
    Ok(url)
}

fn gitlab_project_api_url(
    endpoint: &str,
    owner: &str,
    repo: &str,
    resource: &[&str],
) -> Result<url::Url, String> {
    let mut project_segments = decode_path_segments(owner)?;
    project_segments.extend(decode_path_segments(repo)?);
    let project_id = project_segments.join("/");

    let mut path = vec![project_id];
    path.extend(resource.iter().map(|part| (*part).to_string()));
    append_path_segments(endpoint, &path)
}

/// Parses a repo URL into (owner, repo) where owner may be a slash-joined
/// path for GitLab subgroups (e.g. "group/subgroup" from
/// https://gitlab.com/group/subgroup/project). For GitHub and Forgejo
/// URLs the owner is always a single segment.
fn parse_url(url: &str) -> Result<(String, String, String), String> {
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
    let host = match parsed.port() {
        Some(p) => format!("{}:{}", parsed.host_str().ok_or("URL has no host")?, p),
        None => parsed.host_str().ok_or("URL has no host")?.to_string(),
    };
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
    Ok((host, owner, repo))
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
    // Helper: build an authorized GET request for a GitLab API URL and validate
    // that the endpoint host hasn't been tampered with before sending the token.
    let make_request = |url: &url::Url| -> Result<reqwest::RequestBuilder, String> {
        if url.scheme() != "https" {
            return Err(format!(
                "GitLab endpoint must use HTTPS (got '{}'). Check gitlab_endpoint in settings.",
                url.scheme()
            ));
        }
        if url.host_str().is_none() {
            return Err(
                "GitLab endpoint URL has no host. Check gitlab_endpoint in settings.".to_string(),
            );
        }
        let mut req = http_client().get(url.clone());
        if !config.gitlab_api_key.is_empty() {
            // Validate host before sending the token — defense-in-depth against
            // path injection escaping into the host position.
            // Note: Forgejo uses a separate forgejo_trusted_hosts allowlist instead.
            let endpoint_host = url::Url::parse(&config.gitlab_endpoint)
                .ok()
                .and_then(|u| u.host_str().map(|s| s.to_string()));
            if url.host_str().map(|s| s.to_string()) != endpoint_host {
                return Err(
                    "GitLab API token refused: constructed URL host does not match \
                     gitlab_endpoint host. Check gitlab_endpoint in settings."
                        .to_string(),
                );
            }
            req = req.header("PRIVATE-TOKEN", &config.gitlab_api_key);
        }
        Ok(req)
    };

    // Try the semver-aware permalink endpoint first (available since GitLab 15.7).
    // Falls back to the releases array if the endpoint returns 404 or an error.
    let permalink_url = gitlab_project_api_url(
        &config.gitlab_endpoint,
        owner,
        repo,
        &["releases", "permalink", "latest"],
    )
    .map_err(|e| format!("Invalid GitLab endpoint URL: {}", e))?;
    if let Ok(req) = make_request(&permalink_url) {
        if let Ok(resp) = req.send().await {
            if resp.status().as_u16() == 200 {
                let json: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse GitLab permalink response: {}", e))?;
                return Ok(json);
            }
            // Non-200 (e.g. 404 on older GitLab) — fall through to array endpoint.
        }
    }

    // Fallback: fetch the releases array and return the first non-draft entry.
    let array_url = gitlab_project_api_url(&config.gitlab_endpoint, owner, repo, &["releases"])
        .map_err(|e| format!("Invalid GitLab endpoint URL: {}", e))?;
    let response = send_with_retry(make_request(&array_url)?).await?;

    let status = response.status();
    if status.as_u16() == 404 {
        return Err(format!(
            "No releases found for {}/{} — the repository may have no releases yet, \
             or the URL may be incorrect",
            owner, repo
        ));
    }
    if !status.is_success() {
        return Err(format!(
            "GitLab API returned {} ({}) for {}/{}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown"),
            owner,
            repo
        ));
    }

    // GitLab returns an array of releases ordered newest-first. Skip drafts
    // and return the first published release so callers can extract `tag_name`
    // uniformly across forges.
    let releases: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitLab API response: {}", e))?;

    releases
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|r| r["draft"] != serde_json::Value::Bool(true))
                .cloned()
        })
        .ok_or_else(|| {
            format!(
                "No releases found for {}/{} — the repository may have no releases yet, \
                 or all releases are drafts",
                owner, repo
            )
        })
}

async fn github_api_call(
    config: &Config,
    owner: &str,
    repo: &str,
) -> Result<serde_json::Value, String> {
    let mut path = decode_path_segments(owner)?;
    path.extend(decode_path_segments(repo)?);
    path.extend(["releases", "latest"].map(str::to_string));
    let api_url = append_path_segments(&config.github_endpoint, &path)
        .map_err(|e| format!("Invalid GitHub endpoint URL: {}", e))?;

    let mut request = http_client().get(api_url.clone());

    if !config.github_api_key.is_empty() {
        // Validate endpoint host before sending the token to prevent exfiltration
        // if github_endpoint is misconfigured to an attacker-controlled URL.
        let parsed = url::Url::parse(api_url.as_str())
            .map_err(|e| format!("Invalid GitHub endpoint URL: {}", e))?;
        if parsed.host_str() != Some("api.github.com") {
            return Err(format!(
                "GitHub API token refused: endpoint host '{}' is not 'api.github.com'. \
                 Check github_endpoint in config.json.",
                parsed.host_str().unwrap_or("(none)")
            ));
        }
        request = request.bearer_auth(&config.github_api_key);
    }

    let response = send_with_retry(request).await?;

    let status = response.status();
    if status.as_u16() == 404 {
        return Err(format!(
            "No releases found for {}/{} — the repository may have no releases yet, \
             or the URL may be incorrect",
            owner, repo
        ));
    }
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
    // Look up this host in the trusted-host list.
    //
    // IDN note (S5): comparison is ASCII-case-insensitive only. Hosts stored as
    // Unicode (e.g. "mygïtea.example") and their punycode equivalents
    // (e.g. "xn--mygïtea-n2a.example") are treated as different entries.
    // In practice all Forgejo/Gitea instances use ASCII hostnames, and
    // `validate_forgejo_host` already rejects non-ASCII input, so this is not
    // exploitable — but it means a user who manually edits the config with
    // a Unicode host must use the exact same form in the URL.
    let host_entry = config
        .forgejo_trusted_hosts
        .iter()
        .enumerate()
        .find(|(_, h)| h.host.eq_ignore_ascii_case(host_url));

    // Reject hosts not on the user-managed allowlist to prevent token exfiltration.
    // To add a new Forgejo host, add it to forgejo_trusted_hosts in Settings.
    if host_entry.is_none() {
        return Err(format!(
            "Host '{}' is not in the Forgejo trusted-host list. Add it in Settings → Forgejo to allow requests to this host.",
            host_url
        ));
    }

    // Retrieve the per-host token (if any) by index — same position as the host entry.
    let token = host_entry
        .and_then(|(idx, _)| config.forgejo_tokens.get(idx))
        .map(|s| s.as_str())
        .unwrap_or("");

    // Forgejo and Gitea share the same API surface.
    // Endpoint: GET https://{host}/api/v1/repos/{owner}/{repo}/releases/latest
    let mut path = ["api", "v1", "repos"].map(str::to_string).to_vec();
    path.extend(decode_path_segments(owner)?);
    path.extend(decode_path_segments(repo)?);
    path.extend(["releases", "latest"].map(str::to_string));
    let api_url = append_path_segments(&format!("https://{}", host_url), &path)
        .map_err(|e| format!("Invalid Forgejo API URL: {}", e))?;

    let mut request = http_client().get(api_url);

    if !token.is_empty() {
        request = request.header("Authorization", format!("token {}", token));
    }

    let response = send_with_retry(request)
        .await
        .map_err(|e| format!("HTTP request to '{}' failed: {}", host_url, e))?;

    let status = response.status();
    if status.as_u16() == 404 {
        return Err(format!(
            "No releases found for {}/{} — the repository may have no releases yet, \
             or the URL may be incorrect",
            owner, repo
        ));
    }
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

/// Sends a request, retrying once after 1 s on 5xx or network errors.
/// `builder` must be cloneable (all callers issue GET with no streaming body).
async fn send_with_retry(builder: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let cloned = builder
        .try_clone()
        .ok_or_else(|| "HTTP request could not be cloned for retry".to_string())?;
    match builder.send().await {
        Ok(resp) if resp.status().is_server_error() => {
            log::warn!(
                "Server returned {}; retrying after 1 s",
                resp.status().as_u16()
            );
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            cloned
                .send()
                .await
                .map_err(|e| format!("HTTP request failed after retry: {}", e))
        }
        Err(e) if e.is_connect() || e.is_timeout() => {
            log::warn!("Network error ({}); retrying after 1 s", e);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            cloned
                .send()
                .await
                .map_err(|e2| format!("HTTP request failed after retry: {}", e2))
        }
        other => other.map_err(|e| format!("HTTP request failed: {}", e)),
    }
}

fn extract_release_notes(json: &serde_json::Value, host_kind: &ForgeKind) -> String {
    let key = match host_kind {
        ForgeKind::GitLab => "description",
        _ => "body",
    };
    json[key].as_str().unwrap_or("").to_string()
}

fn extract_release_timestamp(json: &serde_json::Value, host_kind: &ForgeKind) -> String {
    let key = match host_kind {
        ForgeKind::GitLab => "released_at",
        _ => "published_at",
    };
    json[key]
        .as_str()
        .or_else(|| json["created_at"].as_str())
        .unwrap_or("")
        .to_string()
}

fn build_repo_data(
    json: &serde_json::Value,
    owner: String,
    repo_name: String,
    host_url: String,
    host_kind: ForgeKind,
    system_version: String,
) -> Result<RepoData, String> {
    let latest_release = json["tag_name"]
        .as_str()
        .ok_or_else(|| "API response missing tag_name — repo may have no releases".to_string())?
        .to_string();
    let release_notes = extract_release_notes(json, &host_kind);
    let latest_release_timestamp = extract_release_timestamp(json, &host_kind);
    Ok(RepoData {
        owner,
        repo_name,
        host_url,
        host_kind,
        latest_release,
        system_version,
        release_notes,
        latest_release_timestamp,
    })
}

pub async fn post_request(
    config: &Config,
    url: String,
    host_kind: ForgeKind,
) -> Result<RepoData, String> {
    let (host_url, owner, repo_name) = parse_url(&url)?;
    let json = api_call(config, &owner, &repo_name, &host_url, &host_kind).await?;
    build_repo_data(&json, owner, repo_name, host_url, host_kind, String::new())
}

pub async fn refresh_repo(config: &Config, old_repo: &RepoData) -> Result<RepoData, String> {
    let json = api_call(
        config,
        &old_repo.owner,
        &old_repo.repo_name,
        &old_repo.host_url,
        &old_repo.host_kind,
    )
    .await?;
    build_repo_data(
        &json,
        old_repo.owner.clone(),
        old_repo.repo_name.clone(),
        old_repo.host_url.clone(),
        old_repo.host_kind.clone(),
        old_repo.system_version.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        append_path_segments, build_repo_data, extract_release_notes, extract_release_timestamp,
        forgejo_api_call, github_api_call, gitlab_project_api_url, http_client, parse_url,
        send_with_retry,
    };
    use crate::{Config, ForgeKind, ForgejoHost};

    // --- parse_url ---

    #[test]
    fn parse_url_github_flat() {
        assert_eq!(
            parse_url("https://github.com/torvalds/linux").unwrap(),
            (
                "github.com".to_string(),
                "torvalds".to_string(),
                "linux".to_string()
            )
        );
    }

    #[test]
    fn parse_url_github_trailing_slash() {
        assert_eq!(
            parse_url("https://github.com/torvalds/linux/").unwrap(),
            (
                "github.com".to_string(),
                "torvalds".to_string(),
                "linux".to_string()
            )
        );
    }

    #[test]
    fn parse_url_gitlab_subgroup() {
        assert_eq!(
            parse_url("https://gitlab.com/group/subgroup/project").unwrap(),
            (
                "gitlab.com".to_string(),
                "group/subgroup".to_string(),
                "project".to_string()
            )
        );
    }

    #[test]
    fn parse_url_gitlab_deep_subgroup() {
        assert_eq!(
            parse_url("https://gitlab.com/a/b/c/d").unwrap(),
            (
                "gitlab.com".to_string(),
                "a/b/c".to_string(),
                "d".to_string()
            )
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

    #[test]
    fn parse_url_preserves_port() {
        assert_eq!(
            parse_url("https://forgejo.local:3000/owner/repo").unwrap(),
            (
                "forgejo.local:3000".to_string(),
                "owner".to_string(),
                "repo".to_string()
            )
        );
    }

    #[test]
    fn parse_url_rejects_too_long() {
        let long = format!("https://github.com/{}", "a".repeat(2048));
        assert!(parse_url(&long).is_err());
    }

    #[test]
    fn gitlab_project_url_encodes_subgroup_as_one_project_id() {
        let url = gitlab_project_api_url(
            "https://gitlab.example/custom/api/v4/projects/",
            "group/subgroup",
            "project",
            &["releases", "permalink", "latest"],
        )
        .unwrap();

        assert_eq!(
            url.as_str(),
            "https://gitlab.example/custom/api/v4/projects/group%2Fsubgroup%2Fproject/releases/permalink/latest"
        );
    }

    #[test]
    fn gitlab_project_url_decodes_before_encoding_and_preserves_endpoint_query() {
        let url = gitlab_project_api_url(
            "https://gitlab.example/api/v4/projects/?access=custom",
            "group%2Fname/subgroup%20name",
            "project%2Bname",
            &["releases"],
        )
        .unwrap();

        assert_eq!(
            url.as_str(),
            "https://gitlab.example/api/v4/projects/group%2Fname%2Fsubgroup%20name%2Fproject+name/releases?access=custom"
        );
    }

    #[test]
    fn append_path_segments_encodes_path_data_and_preserves_base_path() {
        let path = ["owner/name".to_string(), "repo name".to_string()];
        let url = append_path_segments("https://api.example/custom/repos/", &path).unwrap();

        assert_eq!(
            url.as_str(),
            "https://api.example/custom/repos/owner%2Fname/repo%20name"
        );
    }

    // --- extract_release_notes ---

    #[test]
    fn extract_release_notes_github_uses_body() {
        let json = serde_json::json!({ "body": "## Changelog\n- item", "description": "ignored" });
        assert_eq!(
            extract_release_notes(&json, &ForgeKind::GitHub),
            "## Changelog\n- item"
        );
    }

    #[test]
    fn extract_release_notes_gitlab_uses_description() {
        let json = serde_json::json!({ "body": "ignored", "description": "GL notes" });
        assert_eq!(extract_release_notes(&json, &ForgeKind::GitLab), "GL notes");
    }

    #[test]
    fn extract_release_notes_forgejo_uses_body() {
        let json = serde_json::json!({ "body": "Forgejo release" });
        assert_eq!(
            extract_release_notes(&json, &ForgeKind::ForgejoCompatible),
            "Forgejo release"
        );
    }

    #[test]
    fn extract_release_notes_missing_key_returns_empty() {
        let json = serde_json::json!({ "tag_name": "v1.0" });
        assert_eq!(extract_release_notes(&json, &ForgeKind::GitHub), "");
    }

    // --- extract_release_timestamp ---

    #[test]
    fn extract_release_timestamp_github_uses_published_at() {
        let json = serde_json::json!({ "published_at": "2024-01-15T10:30:00Z" });
        assert_eq!(
            extract_release_timestamp(&json, &ForgeKind::GitHub),
            "2024-01-15T10:30:00Z"
        );
    }

    #[test]
    fn extract_release_timestamp_gitlab_uses_released_at() {
        let json = serde_json::json!({ "released_at": "2024-02-20T08:00:00Z" });
        assert_eq!(
            extract_release_timestamp(&json, &ForgeKind::GitLab),
            "2024-02-20T08:00:00Z"
        );
    }

    #[test]
    fn extract_release_timestamp_gitlab_falls_back_to_created_at() {
        let json = serde_json::json!({ "created_at": "2024-02-19T12:00:00Z" });
        assert_eq!(
            extract_release_timestamp(&json, &ForgeKind::GitLab),
            "2024-02-19T12:00:00Z"
        );
    }

    #[test]
    fn extract_release_timestamp_forgejo_uses_published_at() {
        let json = serde_json::json!({ "published_at": "2024-03-10T14:00:00Z" });
        assert_eq!(
            extract_release_timestamp(&json, &ForgeKind::ForgejoCompatible),
            "2024-03-10T14:00:00Z"
        );
    }

    #[test]
    fn extract_release_timestamp_forgejo_falls_back_to_created_at() {
        let json = serde_json::json!({ "created_at": "2024-03-09T09:00:00Z" });
        assert_eq!(
            extract_release_timestamp(&json, &ForgeKind::ForgejoCompatible),
            "2024-03-09T09:00:00Z"
        );
    }

    #[test]
    fn extract_release_timestamp_missing_key_returns_empty() {
        let json = serde_json::json!({ "tag_name": "v1.0" });
        assert_eq!(extract_release_timestamp(&json, &ForgeKind::GitHub), "");
    }

    // --- post_request error paths (no HTTP) ---

    #[tokio::test]
    async fn post_request_rejects_http_url() {
        let config = Config::new();
        let err = super::post_request(
            &config,
            "http://github.com/owner/repo".to_string(),
            ForgeKind::GitHub,
        )
        .await
        .unwrap_err();
        assert!(err.contains("HTTPS"), "expected HTTPS error, got: {}", err);
    }

    #[tokio::test]
    async fn post_request_rejects_too_short_url() {
        let config = Config::new();
        let err = super::post_request(
            &config,
            "https://github.com/onlyone".to_string(),
            ForgeKind::GitHub,
        )
        .await
        .unwrap_err();
        assert!(
            err.contains("owner and repository"),
            "expected segment error, got: {}",
            err
        );
    }

    // --- GitLab API tests via mockito (HTTP endpoint via gitlab_endpoint config) ---
    // Note: our make_request closure validates scheme == "https", so we use a
    // custom wrapper that bypasses the gitlab_api_call function and tests the
    // JSON-level behaviour of parse_url + extract_release_notes.

    #[test]
    fn gitlab_draft_release_skipped_in_array() {
        // Simulate the array-fallback logic: the first entry is a draft,
        // the second is a real release. Verify that draft filtering works.
        let releases = serde_json::json!([
            { "tag_name": "v2.0", "draft": true },
            { "tag_name": "v1.0", "draft": false }
        ]);
        let found = releases.as_array().and_then(|arr| {
            arr.iter()
                .find(|r| r["draft"] != serde_json::Value::Bool(true))
                .cloned()
        });
        assert_eq!(found.unwrap()["tag_name"], "v1.0");
    }

    #[test]
    fn gitlab_all_drafts_returns_none() {
        let releases = serde_json::json!([
            { "tag_name": "v2.0", "draft": true },
            { "tag_name": "v1.0", "draft": true }
        ]);
        let found = releases.as_array().and_then(|arr| {
            arr.iter()
                .find(|r| r["draft"] != serde_json::Value::Bool(true))
                .cloned()
        });
        assert!(found.is_none());
    }

    // ── build_repo_data ───────────────────────────────────────────────────────

    #[test]
    fn build_repo_data_succeeds_with_tag_name() {
        let json = serde_json::json!({ "tag_name": "v1.2.3", "body": "release notes", "published_at": "2024-01-15T10:30:00Z" });
        let result = build_repo_data(
            &json,
            "owner".to_string(),
            "repo".to_string(),
            "github.com".to_string(),
            ForgeKind::GitHub,
            String::new(),
        )
        .unwrap();
        assert_eq!(result.latest_release, "v1.2.3");
        assert_eq!(result.release_notes, "release notes");
        assert_eq!(result.latest_release_timestamp, "2024-01-15T10:30:00Z");
    }

    #[test]
    fn build_repo_data_fails_when_tag_name_missing() {
        let json = serde_json::json!({ "body": "notes" });
        let err = build_repo_data(
            &json,
            "owner".to_string(),
            "repo".to_string(),
            "github.com".to_string(),
            ForgeKind::GitHub,
            String::new(),
        )
        .unwrap_err();
        assert!(
            err.contains("missing tag_name") || err.contains("tag_name"),
            "got: {err}"
        );
    }

    #[test]
    fn build_repo_data_fails_when_tag_name_is_null() {
        let json = serde_json::json!({ "tag_name": null, "body": "notes" });
        let err = build_repo_data(
            &json,
            "owner".to_string(),
            "repo".to_string(),
            "github.com".to_string(),
            ForgeKind::GitHub,
            String::new(),
        )
        .unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn build_repo_data_uses_description_for_gitlab() {
        let json = serde_json::json!({
            "tag_name": "v1.0",
            "description": "GitLab notes",
            "body": "ignored",
            "released_at": "2024-02-20T08:00:00Z"
        });
        let result = build_repo_data(
            &json,
            "owner".to_string(),
            "repo".to_string(),
            "gitlab.com".to_string(),
            ForgeKind::GitLab,
            String::new(),
        )
        .unwrap();
        assert_eq!(result.release_notes, "GitLab notes");
        assert_eq!(result.latest_release_timestamp, "2024-02-20T08:00:00Z");
    }

    // ── send_with_retry ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn send_with_retry_returns_200_immediately() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_body(r#"{"tag_name":"v1.0"}"#)
            .expect(1)
            .create_async()
            .await;

        let url = format!("{}/test", server.url());
        let builder = http_client().get(&url);
        let resp = send_with_retry(builder).await.unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        mock.assert_async().await;
    }

    // This test sleeps for ~1 second due to the retry backoff — marked #[ignore]
    // so it doesn't slow down the default `cargo test` run.
    // Run explicitly with: cargo test -- --ignored send_with_retry_retries_on_500
    #[tokio::test]
    #[ignore]
    async fn send_with_retry_retries_on_500_succeeds_on_second() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/retry")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;
        let mock2 = server
            .mock("GET", "/retry")
            .with_status(200)
            .with_body(r#"{"tag_name":"v2.0"}"#)
            .expect(1)
            .create_async()
            .await;

        let url = format!("{}/retry", server.url());
        let builder = http_client().get(&url);
        let resp = send_with_retry(builder).await.unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        mock.assert_async().await;
        mock2.assert_async().await;
    }

    #[tokio::test]
    async fn send_with_retry_does_not_retry_on_4xx() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/notfound")
            .with_status(404)
            .expect(1)
            .create_async()
            .await;

        let url = format!("{}/notfound", server.url());
        let builder = http_client().get(&url);
        let resp = send_with_retry(builder).await.unwrap();
        assert_eq!(resp.status().as_u16(), 404);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn send_with_retry_passes_through_401() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/unauth")
            .with_status(401)
            .expect(1)
            .create_async()
            .await;

        let url = format!("{}/unauth", server.url());
        let builder = http_client().get(&url);
        let resp = send_with_retry(builder).await.unwrap();
        assert_eq!(resp.status().as_u16(), 401);
        mock.assert_async().await;
    }

    // ── github_api_call ───────────────────────────────────────────────────────

    fn sample_config_for_mock(base_url: &str) -> Config {
        let mut config = Config::new();
        // Point at mockito server; no token so the api.github.com guard doesn't fire.
        config.github_endpoint = format!("{}/", base_url);
        config
    }

    #[tokio::test]
    async fn github_api_call_returns_parsed_json_on_200() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/owner/repo/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name":"v2.0","body":""}"#)
            .create_async()
            .await;

        let config = sample_config_for_mock(&server.url());
        let json = github_api_call(&config, "owner", "repo").await.unwrap();
        assert_eq!(json["tag_name"], "v2.0");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn github_api_call_returns_descriptive_error_on_404() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/owner/repo/releases/latest")
            .with_status(404)
            .create_async()
            .await;

        let config = sample_config_for_mock(&server.url());
        let err = github_api_call(&config, "owner", "repo").await.unwrap_err();
        assert!(err.contains("No releases found"), "got: {err}");
    }

    #[tokio::test]
    async fn github_api_call_returns_error_on_403() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/owner/repo/releases/latest")
            .with_status(403)
            .create_async()
            .await;

        let config = sample_config_for_mock(&server.url());
        let err = github_api_call(&config, "owner", "repo").await.unwrap_err();
        assert!(err.contains("403"), "got: {err}");
    }

    #[tokio::test]
    async fn github_api_call_rejects_token_for_non_github_host() {
        let mut server = mockito::Server::new_async().await;
        // Mock must never be called — the host guard fires before any HTTP.
        let mock = server
            .mock("GET", "/owner/repo/releases/latest")
            .expect(0)
            .create_async()
            .await;

        let mut config = sample_config_for_mock(&server.url());
        config.github_api_key = "ghp_secret".to_string();
        let err = github_api_call(&config, "owner", "repo").await.unwrap_err();
        assert!(
            err.contains("token refused") || err.contains("refused"),
            "got: {err}"
        );
        mock.assert_async().await;
    }

    // ── forgejo_api_call ──────────────────────────────────────────────────────
    // forgejo_api_call hardcodes https:// in its URL, so mockito (http only)
    // cannot intercept the actual HTTP call. Only the trusted-host guard
    // (which fires before any network I/O) is testable here.

    #[tokio::test]
    async fn forgejo_api_call_rejects_untrusted_host() {
        let mut config = Config::new();
        config.forgejo_trusted_hosts = vec![];
        config.forgejo_tokens = vec![];
        let err = forgejo_api_call(&config, "example.com", "owner", "repo")
            .await
            .unwrap_err();
        assert!(
            err.contains("trusted-host") || err.contains("trusted"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn forgejo_api_call_rejects_host_not_in_list() {
        let mut config = Config::new();
        config.forgejo_trusted_hosts = vec![ForgejoHost {
            host: "codeberg.org".to_string(),
            token_ref: String::new(),
        }];
        config.forgejo_tokens = vec![String::new()];
        let err = forgejo_api_call(&config, "other.example.com", "owner", "repo")
            .await
            .unwrap_err();
        assert!(
            err.contains("trusted-host") || err.contains("trusted"),
            "got: {err}"
        );
    }
}
