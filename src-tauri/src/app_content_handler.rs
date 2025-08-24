use crate::json_handler::JSONHandler;
use serde::Deserialize;
use std::collections::HashMap;
use tracker_libs::RepoData;

#[derive(Debug, Deserialize, PartialEq)]
pub struct AppContentHandler {}
impl AppContentHandler {
    pub fn read_repos(
        filepath: &String,
    ) -> Result<HashMap<String, RepoData>, Box<dyn std::error::Error>> {
        let fp_clone = filepath.clone();
        match JSONHandler::read_from_json::<HashMap<String, RepoData>>(fp_clone) {
            Ok(repos) => Ok(repos),
            e => e,
        }
    }

    pub fn write_to_data_file(datafilepath: &String, repos: &HashMap<String, RepoData>) {
        let fp_clone = datafilepath.clone();
        match JSONHandler::write_json_file::<HashMap<String, RepoData>>(fp_clone, repos) {
            Ok(_) => {}
            Err(e) => eprintln!("Error occurred during repos update: {:?}", e),
        };
    }

    pub fn add_repo(datafilepath: &String, url: String, new_repo_data: RepoData) {
        match AppContentHandler::read_repos(datafilepath) {
            Ok(mut repos) => {
                repos.entry(url).or_insert(new_repo_data);
                AppContentHandler::write_to_data_file(datafilepath, &repos);
            }
            Err(e) => eprintln!("Error occured while adding new repo: {}", e),
        }
    }

    pub fn del_repo(datafilepath: &String, url: &String) {
        match AppContentHandler::read_repos(datafilepath) {
            Ok(mut repos) => {
                repos.remove(url);
                AppContentHandler::write_to_data_file(datafilepath, &repos);
            }
            Err(e) => eprintln!("Error occured while deleting the repo: {}", e),
        }
    }
    pub fn upd_repo(datafilepath: &String, new_data: RepoData, url: &String) {
        let url_clone = url.clone();
        match AppContentHandler::read_repos(datafilepath) {
            Ok(mut repos) => {
                repos.insert(url_clone, new_data).unwrap();
                AppContentHandler::write_to_data_file(datafilepath, &repos);
            }
            Err(e) => eprintln!("Error occured while updating the repo: {}", e),
        }
    }

    pub fn upd_repo_status(datafilepath: &String, url: &String) {
        match AppContentHandler::read_repos(datafilepath) {
            Ok(repos) => match repos.get(url) {
                Some(old_data) => {
                    let new_data: RepoData = RepoData {
                        owner: old_data.owner.clone(),
                        repo_name: old_data.repo_name.clone(),
                        latest_release: old_data.latest_release.clone(),
                        system_version: old_data.latest_release.clone(),
                        notes: old_data.notes.clone(),
                    };
                    AppContentHandler::upd_repo(datafilepath, new_data, url);
                }
                _ => eprintln!("Something went wrong in updating"),
            },
            Err(e) => eprintln!("Error occured while deleting the repo: {}", e),
        }
    }
}
