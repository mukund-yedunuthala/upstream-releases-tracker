mod app_content_handler;
mod config_handler;
mod git_api_handler;
mod helper;
mod json_handler;
mod migration;

use crate::{app_content_handler::AppContentHandler, git_api_handler::GitHandler};
use std::collections::BTreeMap;
use tracker_libs::{Config, RepoData};

static DATAFILE: &str = "upstream-releases-tracker/data/repos.json";

fn datafile_path_string() -> Result<String, String> {
    let data_dir = dirs::data_local_dir()
        .ok_or("Failed to get local data directory")?;

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
async fn get_config() -> Result<Config, String> {
    helper::read_config()
}

#[tauri::command]
async fn get_repos() -> Result<BTreeMap<String, RepoData>, String> {
    let datafile_path_string = datafile_path_string()?;
    AppContentHandler::read_repos(&datafile_path_string)
        .map_err(|e| format!("Failed to read repos: {}", e))
}

#[tauri::command]
async fn delete_repo(url: String) -> Result<(), String> {
    let datafile_path_string = datafile_path_string()?;
    AppContentHandler::del_repo(&datafile_path_string, &url)
        .map_err(|e| format!("Failed to delete repo {}: {}", url, e))
}

#[tauri::command]
async fn refresh_repo(url: String) -> Result<String, String> {
    let datafile_path_string = datafile_path_string()?;

    let mut all_repos = AppContentHandler::read_repos(&datafile_path_string)
        .map_err(|e| format!("Failed to fetch all repos: {}", e))?;

    let repo = all_repos
        .get(&url)
        .ok_or_else(|| format!("Repo not found: {}", url))?;

    let config = helper::read_config()?;  // now propagates
    let git_api_handler = GitHandler {};

    let result = git_api_handler
        .refresh_repo(&config, repo)
        .await
        .map_err(|e| format!("Failed to refresh repo: {}", e))?;

    all_repos.insert(url, result);

    AppContentHandler::write_to_data_file(&datafile_path_string, &all_repos)
        .map_err(|e| format!("Failed to write repos: {}", e))?;

    Ok("Repo refreshed successfully".to_string())
}

#[tauri::command]
async fn add_repo(url: String, host: String) -> Result<String, String> {
    let datafile_path_string = datafile_path_string()?;

    let config = helper::read_config()?;
    let git_api_handler = GitHandler {};

    let new_repo_data = git_api_handler
        .post_request(&config, url.clone(), host)
        .await
        .map_err(|e| format!("Error adding repository: {}", e))?;

    AppContentHandler::add_repo(&datafile_path_string, url, new_repo_data)
        .map_err(|e| format!("Failed to persist new repo: {}", e))?;

    Ok("Repo added successfully".to_string())
}

#[tauri::command]
async fn mark_as_updated(url: String) -> Result<(), String> {
    let datafile_path_string = datafile_path_string()?;
    AppContentHandler::upd_repo_status(&datafile_path_string, &url)
        .map_err(|e| format!("Failed to mark repo as updated {}: {}", url, e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Ok(path) = datafile_path_string() {
            migration::run_migrations(&path);
        }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_repos,
            add_repo,
            delete_repo,
            refresh_repo,
            mark_as_updated,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
