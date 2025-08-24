mod app_content_handler;
mod config_handler;
mod git_api_handler;
mod helper;
mod json_handler;
use crate::app_content_handler::AppContentHandler;
use std::collections::HashMap;
use tracker_libs::{Config, RepoData};

#[tauri::command]
async fn get_config() -> Result<Config, String> {
    Ok(helper::read_config())
}

#[tauri::command]
async fn get_repos() -> Result<HashMap<String, RepoData>, String> {
    let datafile_path = dirs::data_local_dir()
        .unwrap()
        .join("upstream-releases-tracker/data/repos.json");
    let datafile_path_string = String::from(datafile_path.to_str().unwrap());
    match AppContentHandler::read_repos(&datafile_path_string) {
        Ok(repos) => Ok(repos),
        Err(e) => Err(format!("Failed to read repos: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_config, get_repos])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
