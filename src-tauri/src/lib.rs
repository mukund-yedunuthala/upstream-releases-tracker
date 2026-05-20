mod app_content_handler;
mod config_handler;
mod git_api_handler;
mod json_handler;
mod migration;

use crate::git_api_handler::GitHandler;
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::Manager;
use tracker_libs::{Config, ForgeKind, RepoData};

static DATAFILE: &str = "upstream-releases-tracker/data/repos.json";

/// Shared app state: the data-file path, a mutex that serializes all
/// read-modify-write operations on it, and the live runtime config. Config
/// is held under its own mutex so the frontend can update endpoints / push
/// Stronghold-sourced API keys at runtime without restarting the app.
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

// API key fields are runtime-only — they live in Stronghold on disk and in
// the in-memory Config when populated by the frontend. This ensures we never
// write them back to config.json when persisting endpoint changes.
fn scrub_keys(config: &Config) -> Config {
    let mut c = config.clone();
    c.github_api_key = String::new();
    c.gitlab_api_key = String::new();
    c.forgejo_token = String::new();
    c
}

#[tauri::command]
async fn get_repos(
    state: tauri::State<'_, AppState>,
) -> Result<BTreeMap<String, RepoData>, String> {
    let _guard = state.lock.lock().await;
    app_content_handler::read_repos(&state.path)
        .map_err(|e| format!("Failed to read repos: {}", e))
}

#[tauri::command]
async fn delete_repo(
    url: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    app_content_handler::del_repo(&state.path, &url)
        .map_err(|e| format!("Failed to delete repo {}: {}", url, e))
}

#[tauri::command]
async fn refresh_repo(
    url: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
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
    let result = GitHandler {}
        .refresh_repo(&config_snapshot, &repo)
        .await
        .map_err(|e| format!("Failed to refresh repo: {}", e))?;

    // 3. Write back under lock.
    {
        let _guard = state.lock.lock().await;
        let mut all_repos = app_content_handler::read_repos(&state.path)
            .map_err(|e| format!("Failed to read repos for write: {}", e))?;
        all_repos.insert(url, result);
        app_content_handler::write_repos(&state.path, &all_repos)
            .map_err(|e| format!("Failed to write repos: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn add_repo(
    url: String,
    host: String,
    forge: ForgeKind,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // HTTP call first, outside the data lock.
    let config_snapshot = state.config.lock().await.clone();
    let new_repo_data = GitHandler {}
        .post_request(&config_snapshot, url.clone(), host, forge)
        .await
        .map_err(|e| format!("Error adding repository: {}", e))?;

    // Write under the lock.
    let _guard = state.lock.lock().await;
    app_content_handler::add_repo(&state.path, url, new_repo_data)
        .map_err(|e| format!("Failed to persist new repo: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn edit_repo(
    old_url: String,
    new_url: String,
    host: String,
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
    let mut new_repo_data = GitHandler {}
        .post_request(&config_snapshot, new_url.clone(), host, forge)
        .await
        .map_err(|e| format!("Error fetching new repo data: {}", e))?;

    // Carry system_version forward so the user's tracking state is not lost.
    new_repo_data.system_version = old_system_version;

    // 3. Atomically replace old_url with new_url under lock.
    let _guard = state.lock.lock().await;
    app_content_handler::edit_repo(&state.path, &old_url, new_url, new_repo_data)
        .map_err(|e| format!("Failed to update repo: {}", e))
}

#[tauri::command]
async fn mark_as_updated(
    url: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let _guard = state.lock.lock().await;
    app_content_handler::upd_repo_status(&state.path, &url)
        .map_err(|e| format!("Failed to mark repo as updated {}: {}", url, e))
}

#[derive(Serialize)]
struct RefreshAllResult {
    ok: Vec<String>,
    err: Vec<(String, String)>,
}

#[tauri::command]
async fn refresh_all(
    state: tauri::State<'_, AppState>,
) -> Result<RefreshAllResult, String> {
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
    let config = state.config.lock().await.clone();

    let handles: Vec<_> = repos_snapshot
        .iter()
        .map(|(url, repo)| {
            let url = url.clone();
            let repo = repo.clone();
            let config = config.clone();
            tokio::task::spawn(async move {
                (url, GitHandler {}.refresh_repo(&config, &repo).await)
            })
        })
        .collect();

    // 3. Collect results, re-read for any concurrent changes, write once.
    let _guard = state.lock.lock().await;
    let mut all_repos = app_content_handler::read_repos(&state.path)
        .map_err(|e| format!("Failed to read repos for write: {}", e))?;

    let mut ok_urls: Vec<String> = Vec::new();
    let mut err_pairs: Vec<(String, String)> = Vec::new();

    for handle in handles {
        match handle.await {
            Ok((url, Ok(new_data))) => {
                all_repos.insert(url.clone(), new_data);
                ok_urls.push(url);
            }
            Ok((url, Err(e))) => {
                err_pairs.push((url, e));
            }
            Err(e) => {
                err_pairs.push(("(unknown)".to_string(), e.to_string()));
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

// ── New commands for Settings UI ────────────────────────────────────────────

#[derive(Serialize)]
struct Endpoints {
    github_endpoint: String,
    gitlab_endpoint: String,
    forgejo_trusted_hosts: Vec<String>,
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

#[tauri::command]
async fn update_endpoints(
    state: tauri::State<'_, AppState>,
    github_endpoint: String,
    gitlab_endpoint: String,
    forgejo_trusted_hosts: Vec<String>,
) -> Result<(), String> {
    // Update in-memory config and snapshot a persistable (scrubbed) copy.
    let to_persist = {
        let mut cfg = state.config.lock().await;
        cfg.github_endpoint = github_endpoint;
        cfg.gitlab_endpoint = gitlab_endpoint;
        cfg.forgejo_trusted_hosts = forgejo_trusted_hosts;
        scrub_keys(&cfg)
    };
    config_handler::write_config(&to_persist)
        .map_err(|e| format!("Failed to persist endpoints: {}", e))
}

#[tauri::command]
async fn update_api_keys(
    state: tauri::State<'_, AppState>,
    github_api_key: String,
    gitlab_api_key: String,
    forgejo_token: String,
) -> Result<(), String> {
    let mut cfg = state.config.lock().await;
    cfg.github_api_key = github_api_key;
    cfg.gitlab_api_key = gitlab_api_key;
    cfg.forgejo_token = forgejo_token;
    Ok(())
}

/// Returns a per-install random hex-encoded 32-byte key from the OS keyring.
/// On first call the key is generated and stored; subsequent calls retrieve it.
/// The string is passed to `Stronghold.load()` in JS, where the stronghold
/// plugin delivers it to the Argon2id closure as a `&str`. If the keyring is
/// unavailable the command returns an error so the frontend can surface it.
#[tauri::command]
fn get_vault_key() -> Result<String, String> {
    use rand::RngCore;

    const SERVICE: &str = "page.mukundyedunuthala.upstream-releases-tracker";
    const ACCOUNT: &str = "vault-key-v2";

    let entry = keyring::Entry::new(SERVICE, ACCOUNT)
        .map_err(|e| format!("Keyring init failed: {}", e))?;

    match entry.get_password() {
        Ok(stored) => Ok(stored),
        Err(keyring::Error::NoEntry) => {
            let mut bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut bytes);
            let encoded: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            entry
                .set_password(&encoded)
                .map_err(|e| format!("Failed to store vault key in keyring: {}", e))?;
            Ok(encoded)
        }
        Err(e) => Err(format!(
            "Keyring access failed — vault unavailable: {}",
            e
        )),
    }
}

#[tauri::command]
async fn get_logs(app: tauri::AppHandle, limit: usize) -> Result<Vec<String>, String> {
    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("Could not get log dir: {}", e))?;

    if !log_dir.exists() {
        return Ok(vec![]);
    }

    // Pick the most recently modified .log file in the log dir.
    let mut latest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    let entries = std::fs::read_dir(&log_dir)
        .map_err(|e| format!("Failed to read log dir: {}", e))?;
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
        return Ok(vec![]);
    };

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read log file: {}", e))?;
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(limit);
    Ok(lines[start..].iter().map(|s| s.to_string()).collect())
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
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_stronghold::Builder::new(|password| {
                use argon2::{Algorithm, Argon2, Params, Version};
                // password = 32 random bytes from the OS keyring (get_vault_key).
                // Argon2id produces a fixed-size 32-byte key with memory cost so
                // brute-forcing the vault.hold file is expensive even if the raw
                // keyring bytes are exposed.
                let params = Params::new(19456, 2, 1, Some(32)).expect("argon2 params");
                let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
                let salt = b"page.mukundyedunuthala.upstream-releases-tracker:v2";
                let mut out = vec![0u8; 32];
                argon
                    .hash_password_into(password.as_bytes(), salt, &mut out)
                    .expect("argon2 hash");
                out
            })
            .build(),
        )
        .plugin(tauri_plugin_store::Builder::new().build())
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
            update_api_keys,
            get_vault_key,
            get_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
