use crate::json_handler;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tracker_libs::RepoData;

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
    json_handler::read_from_json::<BTreeMap<String, RepoData>>(filepath)
}

pub fn write_repos(
    datafilepath: &str,
    repos: &BTreeMap<String, RepoData>,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_dir(datafilepath)?;
    json_handler::write_json_file::<BTreeMap<String, RepoData>>(datafilepath, repos)
}

pub fn add_repo(
    datafilepath: &str,
    url: String,
    new_repo_data: RepoData,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos = read_repos(datafilepath)?;
    repos.insert(url, new_repo_data);
    write_repos(datafilepath, &repos)
}

pub fn del_repo(datafilepath: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos = read_repos(datafilepath)?;
    repos.remove(url);
    write_repos(datafilepath, &repos)
}

pub fn upd_repo_status(
    datafilepath: &str,
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos = read_repos(datafilepath)?;
    let entry = repos
        .get_mut(url)
        .ok_or_else(|| format!("Repo not found while updating status: {}", url))?;
    entry.system_version = entry.latest_release.clone();
    write_repos(datafilepath, &repos)
}

/// Atomically replace `old_url` with `new_url`, preserving all data from `new_data`
/// (caller is responsible for transferring fields like system_version before calling).
pub fn edit_repo(
    datafilepath: &str,
    old_url: &str,
    new_url: String,
    new_data: RepoData,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos = read_repos(datafilepath)?;
    repos.remove(old_url);
    repos.insert(new_url, new_data);
    write_repos(datafilepath, &repos)
}
