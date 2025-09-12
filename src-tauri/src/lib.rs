mod app_content_handler;
mod config_handler;
mod git_api_handler;
mod helper;
mod json_handler;
use crate::{app_content_handler::AppContentHandler, git_api_handler::GitHandler};
use std::collections::HashMap;
use tracker_libs::{Config, RepoData};
static DATAFILE: &str = "upstream-releases-tracker/data/repos-copy.json";
#[tauri::command]
async fn get_config() -> Result<Config, String> {
    Ok(helper::read_config())
}

#[tauri::command]
async fn get_repos() -> Result<HashMap<String, RepoData>, String> {
    let datafile_path = dirs::data_local_dir().unwrap().join(DATAFILE);
    let datafile_path_string = String::from(datafile_path.to_str().unwrap());
    match AppContentHandler::read_repos(&datafile_path_string) {
        Ok(repos) => Ok(repos),
        Err(e) => Err(format!("Failed to read repos: {}", e)),
    }
}

#[tauri::command]
async fn delete_repo(url: String) {
    let url = url.clone();
    let datafile_path = dirs::data_local_dir().unwrap().join(DATAFILE);
    let datafile_path_string = String::from(datafile_path.to_str().unwrap());
    AppContentHandler::del_repo(&datafile_path_string, &url);
}

#[tauri::command]
async fn refresh_repo(url: String) -> Result<String, String> {
    // Prepare datafile path
    let datafile_path = dirs::data_local_dir()
        .ok_or("Failed to get local data directory")?
        .join(DATAFILE);

    let datafile_path_string = datafile_path
        .to_str()
        .ok_or("Failed to convert datafile path to string")?
        .to_string();

    // Read all repos
    let mut all_repos = AppContentHandler::read_repos(&datafile_path_string)
        .map_err(|e| format!("Failed to fetch all repos: {}", e))?;

    // Find target repo
    let repo = all_repos
        .get(&url)
        .ok_or_else(|| format!("Repo not found: {}", url))?;

    let config = helper::read_config();

    let git_api_handler = GitHandler {};

    // Call refresh_repo, now fully async
    let result = git_api_handler
        .refresh_repo(&config, repo)
        .await
        .map_err(|e| format!("Failed to refresh repo: {}", e))?;
    // Writing data
    all_repos.insert(url, result);
    AppContentHandler::write_to_data_file(&datafile_path_string, &all_repos);

    Ok(format!("Repo refreshed successfully"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_repos,
            delete_repo,
            refresh_repo
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
