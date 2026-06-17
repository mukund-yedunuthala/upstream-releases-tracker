use crate::RepoData;
use crate::json_handler;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

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
    if repos.contains_key(&url) {
        return Err(format!("Repository '{}' is already tracked", url).into());
    }
    repos.insert(url, new_repo_data);
    write_repos(datafilepath, &repos)
}

pub fn del_repo(datafilepath: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos = read_repos(datafilepath)?;
    repos.remove(url);
    write_repos(datafilepath, &repos)
}

pub fn upd_repo_status(datafilepath: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos = read_repos(datafilepath)?;
    let entry = repos
        .get_mut(url)
        .ok_or_else(|| format!("Repo not found while updating status: {}", url))?;
    if entry.latest_release.is_empty() {
        return Err("Cannot mark as updated: latest_release is not yet known".into());
    }
    entry.system_version = entry.latest_release.clone();
    write_repos(datafilepath, &repos)
}

/// Replace `old_url` with `new_url`, preserving all data from `new_data`.
/// The caller is responsible for transferring fields like `system_version` before calling.
///
/// # Atomicity
/// This function performs a single read-modify-write cycle guarded by the
/// caller's `tokio::sync::Mutex` in `lib.rs`. The write itself goes through
/// `json_handler::write_json_file`, which uses a rename-over-temp pattern —
/// so the file on disk is either the old state or the new state, never a
/// partial mix. However, this is **not** crash-atomic at the OS level (the
/// rename can be lost on power failure before the journal flushes). For a
/// single-user desktop app this trade-off is acceptable.
pub fn edit_repo(
    datafilepath: &str,
    old_url: &str,
    new_url: String,
    new_data: RepoData,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos = read_repos(datafilepath)?;
    if new_url != old_url && repos.contains_key(&new_url) {
        return Err(format!("Repository '{}' is already tracked", new_url).into());
    }
    repos.remove(old_url);
    repos.insert(new_url, new_data);
    write_repos(datafilepath, &repos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ForgeKind;
    use tempfile::NamedTempFile;

    /// Returns a NamedTempFile whose underlying file has been removed so that
    /// `read_repos` sees a non-existent path and returns an empty BTreeMap,
    /// while the path itself remains reserved until the returned value is dropped.
    fn tmp_path() -> NamedTempFile {
        let f = NamedTempFile::new().unwrap();
        // Remove the empty file created by NamedTempFile so read_repos won't
        // try to parse empty JSON.  The path string is still valid for our use.
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
        }
    }

    const URL_A: &str = "https://github.com/owner/repo";
    const URL_B: &str = "https://github.com/owner/other";

    #[test]
    fn read_repos_nonexistent_returns_empty() {
        let result = read_repos("/tmp/__upstream_tracker_nonexistent_repos_12345.json").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn add_and_read_round_trip() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        let repos = read_repos(path).unwrap();
        assert!(repos.contains_key(URL_A));
        assert_eq!(repos[URL_A].latest_release, "v1.0");
    }

    #[test]
    fn add_repo_duplicate_returns_error() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        let err = add_repo(path, URL_A.to_string(), sample_repo("v2.0")).unwrap_err();
        assert!(err.to_string().contains("already tracked"));
    }

    #[test]
    fn del_repo_removes_entry() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        del_repo(path, URL_A).unwrap();
        let repos = read_repos(path).unwrap();
        assert!(!repos.contains_key(URL_A));
    }

    #[test]
    fn edit_repo_replaces_url() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        edit_repo(path, URL_A, URL_B.to_string(), sample_repo("v1.0")).unwrap();
        let repos = read_repos(path).unwrap();
        assert!(!repos.contains_key(URL_A));
        assert!(repos.contains_key(URL_B));
    }

    #[test]
    fn edit_repo_collision_returns_error() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        add_repo(path, URL_A.to_string(), sample_repo("v1.0")).unwrap();
        add_repo(path, URL_B.to_string(), sample_repo("v2.0")).unwrap();
        let err = edit_repo(path, URL_A, URL_B.to_string(), sample_repo("v1.0")).unwrap_err();
        assert!(err.to_string().contains("already tracked"));
    }

    #[test]
    fn upd_repo_status_sets_system_version() {
        let file = tmp_path();
        let path = file.path().to_str().unwrap();
        add_repo(path, URL_A.to_string(), sample_repo("v3.0")).unwrap();
        upd_repo_status(path, URL_A).unwrap();
        let repos = read_repos(path).unwrap();
        assert_eq!(repos[URL_A].system_version, "v3.0");
    }
}
