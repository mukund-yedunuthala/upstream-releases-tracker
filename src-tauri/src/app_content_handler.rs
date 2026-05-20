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
        JSONHandler::read_from_json::<BTreeMap<String, RepoData>>(filepath)
    }

    pub fn write_to_data_file(
        datafilepath: &str,
        repos: &BTreeMap<String, RepoData>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::ensure_dir(datafilepath)?;
        JSONHandler::write_json_file::<BTreeMap<String, RepoData>>(datafilepath, repos)
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

    pub fn del_repo(datafilepath: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut repos = Self::read_repos(datafilepath)?;
        repos.remove(url);
        Self::write_to_data_file(datafilepath, &repos)
    }

    pub fn upd_repo_status(
        datafilepath: &str,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut repos = Self::read_repos(datafilepath)?;
        let entry = repos
            .get_mut(url)
            .ok_or_else(|| format!("Repo not found while updating status: {}", url))?;
        entry.system_version = entry.latest_release.clone();
        Self::write_to_data_file(datafilepath, &repos)
    }
}
