// src/models.rs
#[derive(Clone, PartialEq)]
pub struct Repo {
    pub id: u32,
    pub name: String,
    pub url: String,
    pub status: String,
}
