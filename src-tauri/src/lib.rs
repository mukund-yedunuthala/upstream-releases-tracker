mod app_content_handler;
mod config_handler;
mod git_api_handler;
mod json_handler;
mod migration;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::Manager;

static DATAFILE: &str = "upstream-releases-tracker/data/repos.json";

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum ForgeKind {
    GitHub,
    GitLab,
    ForgejoCompatible,
}

impl Default for ForgeKind {
    fn default() -> Self {
        ForgeKind::GitHub
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct ForgejoHost {
    pub host: String,
    pub token_ref: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Config {
    pub github_api_key: String,
    pub github_endpoint: String,
    pub gitlab_api_key: String,
    pub gitlab_endpoint: String,
    #[serde(skip)]
    pub forgejo_tokens: Vec<String>,
    pub forgejo_trusted_hosts: Vec<ForgejoHost>,
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

impl Config {
    pub fn new() -> Self {
        Config {
            github_api_key: String::new(),
            github_endpoint: "https://api.github.com/repos/".to_string(),
            gitlab_api_key: String::new(),
            gitlab_endpoint: "https://gitlab.com/api/v4/projects/".to_string(),
            forgejo_tokens: vec![String::new()],
            forgejo_trusted_hosts: vec![ForgejoHost {
                host: "codeberg.org".to_string(),
                token_ref: "forgejo_token:codeberg.org".to_string(),
            }],
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct RepoData {
    pub owner: String,
    pub repo_name: String,
    pub host_url: String,
    pub host_kind: ForgeKind,
    pub latest_release: String,
    pub system_version: String,
    #[serde(default)]
    pub release_notes: String,
    #[serde(default)]
    pub latest_release_timestamp: String,
}

/// Shared data-file and runtime configuration state.
struct AppState {
    path: String,
    lock: tokio::sync::Mutex<()>,
    config: tokio::sync::Mutex<Config>,
}

fn datafile_path_string() -> Result<String, String> {
    let data_dir = dirs::data_local_dir().ok_or("Failed to get local data directory")?;
    let datafile_path = data_dir.join(DATAFILE);
    if let Some(parent) = datafile_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }
    Ok(datafile_path
        .to_str()
        .ok_or("Failed to convert datafile path to string")
        .map(|s| s.to_string())?)
}

/// Persist a refresh only if the repo still exists.
fn merge_refresh_result(
    path: &str,
    url: &str,
    new_data: RepoData,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos = app_content_handler::read_repos(path)?;
    if !repos.contains_key(url) {
        return Err(format!("Repo '{}' was removed during refresh", url).into());
    }
    repos.insert(url.to_string(), new_data);
    app_content_handler::write_repos(path, &repos)
}

// API keys are runtime-only and must not be written to config.json.
fn scrub_keys(config: &Config) -> Config {
    let mut c = config.clone();
    c.github_api_key = String::new();
    c.gitlab_api_key = String::new();
    c.forgejo_tokens = vec![String::new(); c.forgejo_trusted_hosts.len()];
    c
}

#[tauri::command]
async fn get_repos(
    state: tauri::State<'_, AppState>,
) -> Result<BTreeMap<String, RepoData>, String> {
    let _guard = state.lock.lock().await;
    app_content_handler::read_repos(&state.path).map_err(|e| {
        log::error!("get_repos failed: {}", e);
        format!("Failed to read repos: {}", e)
    })
}

#[tauri::command]
async fn delete_repo(url: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    app_content_handler::del_repo(&state.path, &url).map_err(|e| {
        log::error!("delete_repo failed for {}: {}", url, e);
        format!("Failed to delete repo {}: {}", url, e)
    })
}

#[tauri::command]
async fn refresh_repo(url: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // 1. Read the target repo under lock, then release before HTTP.
    let repo = {
        let _guard = state.lock.lock().await;
        let all_repos = app_content_handler::read_repos(&state.path)
            .map_err(|e| format!("Failed to read repos: {}", e))?;
        all_repos
            .get(&url)
            .cloned()
            .ok_or_else(|| format!("Repo not found: {}", url))?
    };

    // 2. Snapshot the config separately, then HTTP call without any lock held.
    let config_snapshot = state.config.lock().await.clone();
    let result = git_api_handler::refresh_repo(&config_snapshot, &repo)
        .await
        .map_err(|e| {
            log::error!("refresh_repo HTTP failed for {}: {}", url, e);
            format!("Failed to refresh repo: {}", e)
        })?;

    // 3. Write back under lock — but only if the repo wasn't deleted while we
    //    were waiting for the HTTP response (guards against TOCTOU with delete).
    {
        let _guard = state.lock.lock().await;
        merge_refresh_result(&state.path, &url, result)
            .map_err(|e| format!("Failed to write refreshed repo: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn add_repo(
    url: String,
    forge: ForgeKind,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // HTTP call first, outside the data lock.
    let config_snapshot = state.config.lock().await.clone();
    let new_repo_data = git_api_handler::post_request(&config_snapshot, url.clone(), forge)
        .await
        .map_err(|e| {
            log::error!("add_repo HTTP failed for {}: {}", url, e);
            format!("Error adding repository: {}", e)
        })?;

    // Write under the lock.
    let _guard = state.lock.lock().await;
    app_content_handler::add_repo(&state.path, url, new_repo_data).map_err(|e| {
        log::error!("add_repo persist failed: {}", e);
        format!("Failed to persist new repo: {}", e)
    })?;

    Ok(())
}

#[tauri::command]
async fn edit_repo(
    old_url: String,
    new_url: String,
    forge: ForgeKind,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // 1. Read old system_version under lock, then release.
    let old_system_version = {
        let _guard = state.lock.lock().await;
        let repos = app_content_handler::read_repos(&state.path)
            .map_err(|e| format!("Failed to read repos: {}", e))?;
        repos
            .get(&old_url)
            .map(|r| r.system_version.clone())
            .unwrap_or_default()
    };

    // 2. HTTP call for the new URL — no lock held.
    let config_snapshot = state.config.lock().await.clone();
    let mut new_repo_data = git_api_handler::post_request(&config_snapshot, new_url.clone(), forge)
        .await
        .map_err(|e| {
            log::error!("edit_repo HTTP failed for {}: {}", new_url, e);
            format!("Error fetching new repo data: {}", e)
        })?;

    // Carry system_version forward so the user's tracking state is not lost.
    new_repo_data.system_version = old_system_version;

    // 3. Atomically replace old_url with new_url under lock.
    let _guard = state.lock.lock().await;
    app_content_handler::edit_repo(&state.path, &old_url, new_url, new_repo_data).map_err(|e| {
        log::error!("edit_repo persist failed: {}", e);
        format!("Failed to update repo: {}", e)
    })
}

#[tauri::command]
async fn mark_as_updated(url: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    app_content_handler::upd_repo_status(&state.path, &url).map_err(|e| {
        log::error!("mark_as_updated failed for {}: {}", url, e);
        format!("Failed to mark repo as updated {}: {}", url, e)
    })
}

#[derive(Serialize)]
struct RefreshAllResult {
    ok: Vec<String>,
    err: Vec<(String, String)>,
}

#[tauri::command]
async fn refresh_all(state: tauri::State<'_, AppState>) -> Result<RefreshAllResult, String> {
    // 1. Snapshot repos under lock, then release.
    let repos_snapshot = {
        let _guard = state.lock.lock().await;
        app_content_handler::read_repos(&state.path)
            .map_err(|e| format!("Failed to read repos: {}", e))?
    };

    if repos_snapshot.is_empty() {
        return Ok(RefreshAllResult {
            ok: vec![],
            err: vec![],
        });
    }

    // 2. Parallel HTTP calls — no lock held during network I/O.
    //    buffer_unordered(8) caps concurrent requests so a large list doesn't
    //    DoS a small Forgejo host or exhaust the GitHub rate-limit budget.
    let config = state.config.lock().await.clone();

    use futures::stream::{self, StreamExt};
    let tasks: Vec<_> = repos_snapshot
        .into_iter()
        .map(|(url, repo)| {
            let config = config.clone();
            async move { (url, git_api_handler::refresh_repo(&config, &repo).await) }
        })
        .collect();
    let results: Vec<_> = stream::iter(tasks).buffer_unordered(8).collect().await;

    // 3. Collect results, re-read for any concurrent changes, write once.
    let _guard = state.lock.lock().await;
    let mut all_repos = app_content_handler::read_repos(&state.path)
        .map_err(|e| format!("Failed to read repos for write: {}", e))?;

    let mut ok_urls: Vec<String> = Vec::new();
    let mut err_pairs: Vec<(String, String)> = Vec::new();

    for (url, outcome) in results {
        match outcome {
            Ok(new_data) => {
                // Only update if the repo still exists — silently skip repos
                // deleted while their HTTP call was in flight (TOCTOU guard).
                if all_repos.contains_key(&url) {
                    all_repos.insert(url.clone(), new_data);
                    ok_urls.push(url);
                }
            }
            Err(e) => {
                log::error!("refresh_all: failed to refresh {}: {}", url, e);
                err_pairs.push((url, e));
            }
        }
    }

    app_content_handler::write_repos(&state.path, &all_repos)
        .map_err(|e| format!("Failed to write repos after refresh: {}", e))?;

    Ok(RefreshAllResult {
        ok: ok_urls,
        err: err_pairs,
    })
}

#[derive(Serialize)]
struct Endpoints {
    github_endpoint: String,
    gitlab_endpoint: String,
    forgejo_trusted_hosts: Vec<ForgejoHost>,
}

#[tauri::command]
async fn get_endpoints(state: tauri::State<'_, AppState>) -> Result<Endpoints, String> {
    let cfg = state.config.lock().await;
    Ok(Endpoints {
        github_endpoint: cfg.github_endpoint.clone(),
        gitlab_endpoint: cfg.gitlab_endpoint.clone(),
        forgejo_trusted_hosts: cfg.forgejo_trusted_hosts.clone(),
    })
}

fn validate_endpoint_url(url: &str, field: &str) -> Result<(), String> {
    if url.len() > 2048 {
        return Err(format!(
            "{} exceeds maximum length of 2048 characters",
            field
        ));
    }
    let parsed =
        url::Url::parse(url).map_err(|e| format!("{} is not a valid URL: {}", field, e))?;
    if parsed.scheme() != "https" {
        return Err(format!(
            "{} must use HTTPS (got '{}')",
            field,
            parsed.scheme()
        ));
    }
    if parsed.host_str().is_none() {
        return Err(format!("{} has no host", field));
    }
    Ok(())
}

fn validate_forgejo_host(host: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err("Forgejo host entry must not be empty".to_string());
    }
    if host.len() > 253 {
        return Err(format!(
            "Forgejo host '{}' exceeds maximum length of 253 characters",
            host
        ));
    }
    if !host.is_ascii() {
        return Err(format!("Forgejo host '{}' must be ASCII-only", host));
    }
    if host.contains('/') || host.contains(':') {
        return Err(format!(
            "Forgejo host '{}' must be a bare hostname (no scheme or path)",
            host
        ));
    }
    Ok(())
}

#[tauri::command]
async fn update_endpoints(
    state: tauri::State<'_, AppState>,
    github_endpoint: String,
    gitlab_endpoint: String,
    forgejo_trusted_hosts: Vec<ForgejoHost>,
) -> Result<(), String> {
    validate_endpoint_url(&github_endpoint, "github_endpoint")?;
    validate_endpoint_url(&gitlab_endpoint, "gitlab_endpoint")?;
    for entry in &forgejo_trusted_hosts {
        validate_forgejo_host(&entry.host)?;
        if entry.token_ref.len() > 512 {
            return Err(format!(
                "token_ref for host '{}' exceeds 512 characters",
                entry.host
            ));
        }
        if entry.token_ref.chars().any(|c| c.is_ascii_control()) {
            return Err(format!(
                "token_ref for host '{}' contains control characters",
                entry.host
            ));
        }
    }

    let (to_persist, changed) = {
        let mut cfg = state.config.lock().await;
        let new_len = forgejo_trusted_hosts.len();
        cfg.forgejo_tokens.resize(new_len, String::new());
        let changed = cfg.github_endpoint != github_endpoint
            || cfg.gitlab_endpoint != gitlab_endpoint
            || cfg.forgejo_trusted_hosts != forgejo_trusted_hosts;
        cfg.github_endpoint = github_endpoint;
        cfg.gitlab_endpoint = gitlab_endpoint;
        cfg.forgejo_trusted_hosts = forgejo_trusted_hosts;
        (scrub_keys(&cfg), changed)
    };

    if changed {
        config_handler::write_config(&to_persist)
            .map_err(|e| format!("Failed to persist endpoints: {}", e))?;
    }
    Ok(())
}

fn sanitize_token(token: &str, field: &str) -> Result<String, String> {
    if token.len() > 4096 {
        return Err(format!(
            "{} exceeds maximum length of 4096 characters",
            field
        ));
    }
    let trimmed = token.trim();
    if trimmed.chars().any(|c| c.is_ascii_control()) {
        return Err(format!("{} contains control characters", field));
    }
    Ok(trimmed.to_string())
}

#[derive(Serialize)]
struct ApiKeys {
    github_api_key: String,
    gitlab_api_key: String,
    forgejo_tokens: Vec<String>,
}

const KEYRING_SERVICE: &str = "page.mukundyedunuthala.upstream-releases-tracker";

fn keyring_account(name: &str) -> String {
    format!("api-key:{}", name)
}

fn read_keyring_secret(name: &str) -> Result<String, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account(name))
        .map_err(|e| format!("Keyring init failed for {}: {}", name, e))?;
    match entry.get_password() {
        Ok(value) => Ok(value),
        Err(keyring::Error::NoEntry) => Ok(String::new()),
        Err(e) => Err(format!("Keyring read failed for {}: {}", name, e)),
    }
}

fn write_keyring_secret(name: &str, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account(name))
        .map_err(|e| format!("Keyring init failed for {}: {}", name, e))?;
    if value.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Keyring delete failed for {}: {}", name, e)),
        }
    } else {
        entry
            .set_password(value)
            .map_err(|e| format!("Keyring write failed for {}: {}", name, e))
    }
}

#[tauri::command]
async fn get_api_keys(state: tauri::State<'_, AppState>) -> Result<ApiKeys, String> {
    let hosts = state.config.lock().await.forgejo_trusted_hosts.clone();
    Ok(ApiKeys {
        github_api_key: read_keyring_secret("github_api_key")?,
        gitlab_api_key: read_keyring_secret("gitlab_api_key")?,
        forgejo_tokens: hosts
            .iter()
            .map(|h| read_keyring_secret(&h.token_ref))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

#[tauri::command]
async fn update_api_keys(
    state: tauri::State<'_, AppState>,
    github_api_key: String,
    gitlab_api_key: String,
    forgejo_tokens: Vec<String>,
) -> Result<(), String> {
    let github_api_key = sanitize_token(&github_api_key, "github_api_key")?;
    let gitlab_api_key = sanitize_token(&gitlab_api_key, "gitlab_api_key")?;
    let forgejo_tokens: Result<Vec<String>, String> = forgejo_tokens
        .into_iter()
        .enumerate()
        .map(|(i, t)| sanitize_token(&t, &format!("forgejo_tokens[{}]", i)))
        .collect();
    let forgejo_tokens = forgejo_tokens?;
    let mut cfg = state.config.lock().await;
    write_keyring_secret("github_api_key", &github_api_key)?;
    write_keyring_secret("gitlab_api_key", &gitlab_api_key)?;
    for (entry, token) in cfg.forgejo_trusted_hosts.iter().zip(&forgejo_tokens) {
        write_keyring_secret(&entry.token_ref, token)?;
    }
    cfg.github_api_key = github_api_key;
    cfg.gitlab_api_key = gitlab_api_key;
    cfg.forgejo_tokens = forgejo_tokens;
    Ok(())
}

#[derive(Serialize)]
struct LogChunk {
    lines: Vec<String>,
    next_offset: u64,
}

#[tauri::command]
async fn get_logs(
    app: tauri::AppHandle,
    limit: usize,
    after_bytes: Option<u64>,
) -> Result<LogChunk, String> {
    use std::io::{BufRead, Read, Seek, SeekFrom};

    const MAX_READ_BYTES: u64 = 262_144; // 256 KiB tail window
    const MAX_LIMIT: usize = 1000;
    let limit = limit.min(MAX_LIMIT);

    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("Could not get log dir: {}", e))?;

    if !log_dir.exists() {
        return Ok(LogChunk {
            lines: vec![],
            next_offset: 0,
        });
    }

    // Pick the most recently modified .log file in the log dir.
    let mut latest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    let entries =
        std::fs::read_dir(&log_dir).map_err(|e| format!("Failed to read log dir: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("log") {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                match &latest {
                    None => latest = Some((path, modified)),
                    Some((_, prev)) if modified > *prev => latest = Some((path, modified)),
                    _ => {}
                }
            }
        }
    }

    let Some((path, _)) = latest else {
        return Ok(LogChunk {
            lines: vec![],
            next_offset: 0,
        });
    };

    let file_len = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to stat log file: {}", e))?
        .len();

    let mut file =
        std::fs::File::open(&path).map_err(|e| format!("Failed to open log file: {}", e))?;

    // Incremental mode: if the caller supplied a byte offset and the file has
    // grown since then, seek to that offset and return only new lines.
    // If the offset is >= file_len the file hasn't grown; return nothing.
    // If the offset > file_len the file was rotated; fall through to full read.
    if let Some(offset) = after_bytes {
        if offset < file_len {
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| format!("Failed to seek log file: {}", e))?;
            let mut buf = String::new();
            std::io::BufReader::new(&mut file)
                .read_to_string(&mut buf)
                .map_err(|e| format!("Failed to read log file: {}", e))?;
            let all_lines: Vec<&str> = buf.lines().collect();
            let start = all_lines.len().saturating_sub(limit);
            return Ok(LogChunk {
                lines: all_lines[start..].iter().map(|s| s.to_string()).collect(),
                next_offset: file_len,
            });
        } else if offset == file_len {
            // No new data.
            return Ok(LogChunk {
                lines: vec![],
                next_offset: file_len,
            });
        }
        // offset > file_len → rotation detected, fall through to full read.
    }

    // Full read: tail the last MAX_READ_BYTES of the file.
    if file_len > MAX_READ_BYTES {
        file.seek(SeekFrom::End(-(MAX_READ_BYTES as i64)))
            .map_err(|e| format!("Failed to seek log file: {}", e))?;
        // Discard the first (potentially partial) line after the seek.
        let mut discard = String::new();
        let mut reader = std::io::BufReader::new(&mut file);
        reader
            .read_line(&mut discard)
            .map_err(|e| format!("Failed to skip partial line: {}", e))?;
        let mut buf = String::new();
        reader
            .read_to_string(&mut buf)
            .map_err(|e| format!("Failed to read log file: {}", e))?;
        let lines: Vec<&str> = buf.lines().collect();
        let start = lines.len().saturating_sub(limit);
        Ok(LogChunk {
            lines: lines[start..].iter().map(|s| s.to_string()).collect(),
            next_offset: file_len,
        })
    } else {
        let mut buf = String::new();
        std::io::BufReader::new(file)
            .read_to_string(&mut buf)
            .map_err(|e| format!("Failed to read log file: {}", e))?;
        let lines: Vec<&str> = buf.lines().collect();
        let start = lines.len().saturating_sub(limit);
        Ok(LogChunk {
            lines: lines[start..].iter().map(|s| s.to_string()).collect(),
            next_offset: file_len,
        })
    }
}

#[tauri::command]
async fn clear_logs(app: tauri::AppHandle) -> Result<(), String> {
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("Could not get log dir: {}", e))?;

    if !log_dir.exists() {
        return Ok(());
    }

    let entries =
        std::fs::read_dir(&log_dir).map_err(|e| format!("Failed to read log dir: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("log") {
            std::fs::File::create(&path)
                .map_err(|e| format!("Failed to clear log file {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_path = datafile_path_string()
        .expect("Failed to determine data directory — cannot start the application");

    migration::run_migrations(&data_path);

    let config_path = config_handler::config_path_string()
        .expect("Failed to determine config directory — cannot start the application");
    migration::run_config_migrations(&config_path);

    let config = config_handler::read_config().unwrap_or_else(|e| {
        eprintln!(
            "Warning: failed to load config ({}). Starting with defaults.",
            e
        );
        Config::default()
    });

    tauri::Builder::default()
        .manage(AppState {
            path: data_path,
            lock: tokio::sync::Mutex::new(()),
            config: tokio::sync::Mutex::new(config),
        })
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(2_097_152)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .format(|out, message, record| {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0);
                    let line = serde_json::json!({
                        "ts": ts,
                        "level": record.level().to_string(),
                        "target": record.target(),
                        "msg": message.to_string(),
                    });
                    out.finish(format_args!("{}", line))
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_repos,
            add_repo,
            edit_repo,
            delete_repo,
            refresh_repo,
            refresh_all,
            mark_as_updated,
            get_endpoints,
            update_endpoints,
            get_api_keys,
            update_api_keys,
            get_logs,
            clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn tmp_path() -> NamedTempFile {
        let f = NamedTempFile::new().unwrap();
        std::fs::remove_file(f.path()).ok();
        f
    }

    fn sample_repo(tag: &str) -> RepoData {
        RepoData {
            owner: "owner".to_string(),
            repo_name: "repo".to_string(),
            host_url: "github.com".to_string(),
            host_kind: ForgeKind::GitHub,
            latest_release: tag.to_string(),
            system_version: String::new(),
            release_notes: String::new(),
            latest_release_timestamp: String::new(),
        }
    }

    const URL_A: &str = "https://github.com/owner/repo";
    const URL_B: &str = "https://github.com/owner/other";

    // ── validate_endpoint_url ─────────────────────────────────────────────────

    #[test]
    fn validate_endpoint_url_accepts_valid_https() {
        assert!(validate_endpoint_url("https://api.github.com/repos/", "github").is_ok());
    }

    #[test]
    fn validate_endpoint_url_rejects_http() {
        let err = validate_endpoint_url("http://api.github.com/repos/", "github").unwrap_err();
        assert!(err.contains("HTTPS"), "expected HTTPS mention, got: {err}");
    }

    #[test]
    fn validate_endpoint_url_rejects_unparseable() {
        let err = validate_endpoint_url("not a url", "github").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn validate_endpoint_url_rejects_no_host() {
        let err = validate_endpoint_url("https://", "github").unwrap_err();
        assert!(
            err.contains("no host") || err.contains("host"),
            "got: {err}"
        );
    }

    #[test]
    fn validate_endpoint_url_rejects_too_long() {
        let url = format!("https://a.com/{}", "b".repeat(2049));
        let err = validate_endpoint_url(&url, "github").unwrap_err();
        assert!(err.contains("2048"), "got: {err}");
    }

    #[test]
    fn validate_endpoint_url_accepts_exactly_2048_chars() {
        // Build: "https://a.com/" + padding to hit exactly 2048 total chars.
        let prefix = "https://a.com/";
        let padding = "b".repeat(2048 - prefix.len());
        let url = format!("{}{}", prefix, padding);
        assert_eq!(url.len(), 2048);
        assert!(validate_endpoint_url(&url, "github").is_ok());
    }

    // ── validate_forgejo_host ─────────────────────────────────────────────────

    #[test]
    fn validate_forgejo_host_accepts_plain_hostname() {
        assert!(validate_forgejo_host("codeberg.org").is_ok());
    }

    #[test]
    fn validate_forgejo_host_rejects_empty() {
        assert!(validate_forgejo_host("").is_err());
    }

    #[test]
    fn validate_forgejo_host_rejects_too_long() {
        let host = "a".repeat(254);
        let err = validate_forgejo_host(&host).unwrap_err();
        assert!(err.contains("253"), "got: {err}");
    }

    #[test]
    fn validate_forgejo_host_rejects_non_ascii() {
        let err = validate_forgejo_host("mygïtea.example").unwrap_err();
        assert!(err.contains("ASCII"), "got: {err}");
    }

    #[test]
    fn validate_forgejo_host_rejects_slash() {
        let err = validate_forgejo_host("example.com/path").unwrap_err();
        assert!(
            err.contains("scheme or path") || err.contains("path"),
            "got: {err}"
        );
    }

    #[test]
    fn validate_forgejo_host_rejects_colon() {
        let err = validate_forgejo_host("example.com:3000").unwrap_err();
        assert!(!err.is_empty(), "got: {err}");
    }

    // ── sanitize_token ────────────────────────────────────────────────────────

    #[test]
    fn sanitize_token_accepts_clean_token() {
        assert_eq!(
            sanitize_token("ghp_abc123", "github").unwrap(),
            "ghp_abc123"
        );
    }

    #[test]
    fn sanitize_token_trims_whitespace() {
        assert_eq!(sanitize_token("  ghp_abc  ", "github").unwrap(), "ghp_abc");
    }

    #[test]
    fn sanitize_token_rejects_too_long() {
        let token = "x".repeat(4097);
        let err = sanitize_token(&token, "github").unwrap_err();
        assert!(err.contains("4096"), "got: {err}");
    }

    #[test]
    fn sanitize_token_rejects_control_characters() {
        let err = sanitize_token("ghp_\x01abc", "github").unwrap_err();
        assert!(err.contains("control"), "got: {err}");
    }

    #[test]
    fn sanitize_token_accepts_empty_string() {
        assert_eq!(sanitize_token("", "github").unwrap(), "");
    }

    // ── merge_refresh_result ──────────────────────────────────────────────────

    #[test]
    fn merge_refresh_result_succeeds_when_repo_exists() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        app_content_handler::add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        merge_refresh_result(path, URL_A, sample_repo("v2.0")).unwrap();
        let repos = app_content_handler::read_repos(path).unwrap();
        assert_eq!(repos[URL_A].latest_release, "v2.0");
    }

    #[test]
    fn merge_refresh_result_fails_when_repo_was_deleted() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        app_content_handler::add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        app_content_handler::del_repo(path, URL_A).unwrap();
        let err = merge_refresh_result(path, URL_A, sample_repo("v2.0")).unwrap_err();
        assert!(
            err.to_string().contains("removed during refresh"),
            "got: {err}"
        );
    }

    #[test]
    fn merge_refresh_result_does_not_modify_other_repos() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        app_content_handler::add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        app_content_handler::add_repo(path, URL_B.to_string(), sample_repo("v9.0")).unwrap();
        merge_refresh_result(path, URL_A, sample_repo("v2.0")).unwrap();
        let repos = app_content_handler::read_repos(path).unwrap();
        assert_eq!(repos[URL_B].latest_release, "v9.0");
    }

    #[test]
    fn merge_refresh_result_overwrites_existing_data() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        app_content_handler::add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        merge_refresh_result(path, URL_A, sample_repo("v2.0")).unwrap();
        let repos = app_content_handler::read_repos(path).unwrap();
        assert_eq!(repos[URL_A].latest_release, "v2.0");
    }

    #[test]
    fn merge_refresh_result_fails_when_file_absent_and_url_not_found() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        // File does not exist → read_repos returns empty map → URL is absent.
        let err = merge_refresh_result(path, URL_A, sample_repo("v1.0")).unwrap_err();
        assert!(
            err.to_string().contains("removed during refresh"),
            "got: {err}"
        );
    }

    #[test]
    fn merge_refresh_result_preserves_system_version() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        let mut initial = sample_repo("v1.0");
        initial.system_version = "v1.0".to_string();
        app_content_handler::add_repo(path, URL_A.to_string(), initial).unwrap();

        let mut updated = sample_repo("v2.0");
        updated.system_version = "v1.0".to_string(); // caller carries it forward
        merge_refresh_result(path, URL_A, updated).unwrap();

        let repos = app_content_handler::read_repos(path).unwrap();
        assert_eq!(repos[URL_A].system_version, "v1.0");
        assert_eq!(repos[URL_A].latest_release, "v2.0");
    }
}
