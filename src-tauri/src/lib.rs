mod app_content_handler;
mod config_handler;
mod git_api_handler;
mod json_handler;
mod migration;

use crate::git_api_handler::GitHandler;
use serde::Serialize;
use std::collections::BTreeMap;
use tracker_libs::{Config, ForgeKind, RepoData};

static DATAFILE: &str = "upstream-releases-tracker/data/repos.json";

/// Shared app state: the data-file path, a mutex that serializes all
/// read-modify-write operations on it, and the config loaded once at startup.
/// HTTP calls must happen *outside* the data lock so the app stays responsive.
struct AppState {
    path: String,
    lock: tokio::sync::Mutex<()>,
    config: Config,
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

    // 2. HTTP call — lock is not held during network I/O.
    let result = GitHandler {}
        .refresh_repo(&state.config, &repo)
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
    // HTTP call first, outside the lock.
    let new_repo_data = GitHandler {}
        .post_request(&state.config, url.clone(), host, forge)
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
        repos.get(&old_url).map(|r| r.system_version.clone()).unwrap_or_default()
    };

    // 2. HTTP call for the new URL — lock is not held.
    let mut new_repo_data = GitHandler {}
        .post_request(&state.config, new_url.clone(), host, forge)
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
        return Ok(RefreshAllResult { ok: vec![], err: vec![] });
    }

    // 2. Parallel HTTP calls — lock is not held during network I/O.
    let config = state.config.clone();

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_path = datafile_path_string()
        .expect("Failed to determine data directory — cannot start the application");

    migration::run_migrations(&data_path);

    let config_path = config_handler::config_path_string()
        .expect("Failed to determine config directory — cannot start the application");
    migration::run_config_migrations(&config_path);

    let config = config_handler::read_config().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config ({}). Starting with defaults.", e);
        Config::default()
    });

    tauri::Builder::default()
        .manage(AppState {
            path: data_path,
            lock: tokio::sync::Mutex::new(()),
            config,
        })
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_repos,
            add_repo,
            edit_repo,
            delete_repo,
            refresh_repo,
            refresh_all,
            mark_as_updated,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
