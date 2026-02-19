use crate::json_handler::JSONHandler;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tracker_libs::RepoData;

#[derive(Debug, Deserialize, PartialEq)]
pub struct AppContentHandler {}

impl AppContentHandler {
    fn ensure_dir(filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(filepath);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    pub fn read_repos(
        filepath: &str,
    ) -> Result<BTreeMap<String, RepoData>, Box<dyn std::error::Error>> {
        let path = Path::new(filepath);
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        JSONHandler::read_from_json::<BTreeMap<String, RepoData>>(filepath.to_string())
    }

    pub fn write_to_data_file(
        datafilepath: &str,
        repos: &BTreeMap<String, RepoData>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::ensure_dir(datafilepath)?;
        JSONHandler::write_json_file::<BTreeMap<String, RepoData>>(
            datafilepath.to_string(),
            repos,
        )
    }

    pub fn add_repo(
        datafilepath: &str,
        url: String,
        new_repo_data: RepoData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut repos = Self::read_repos(datafilepath)?;
        repos.insert(url, new_repo_data);
        Self::write_to_data_file(datafilepath, &repos)
    }

    pub fn del_repo(
        datafilepath: &str,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut repos = Self::read_repos(datafilepath)?;
        repos.remove(url);
        Self::write_to_data_file(datafilepath, &repos)
    }

    pub fn upd_repo(
        datafilepath: &str,
        new_data: RepoData,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut repos = Self::read_repos(datafilepath)?;
        repos.insert(url.to_string(), new_data);
        Self::write_to_data_file(datafilepath, &repos)
    }

    pub fn upd_repo_status(
        datafilepath: &str,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repos = Self::read_repos(datafilepath)?;

        if let Some(old_data) = repos.get(url) {
            let new_data = RepoData {
                owner: old_data.owner.clone(),
                repo_name: old_data.repo_name.clone(),
                latest_release: old_data.latest_release.clone(),
                system_version: old_data.latest_release.clone(),
                notes: old_data.notes.clone(),
            };
            Self::upd_repo(datafilepath, new_data, url)?;
        } else {
            return Err(format!("Repo not found while updating status: {}", url).into());
        }

        Ok(())
    }
}
