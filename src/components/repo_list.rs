// src/components/repo_list.rs
use crate::models::Repo;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct RepoListProps {
    pub repos: Vec<Repo>,
}

#[function_component(RepoList)]
pub fn repo_list(props: &RepoListProps) -> Html {
    let repos = &props.repos;

    html! {
        <div class="repo-list">
            <h2>{"Repositories"}</h2>
            if repos.is_empty() {
                <p>{"No repositories found."}</p>
            } else {
                <ul>
                    {for repos.iter().map(|repo| html!{
                        <li key={repo.id}>
                            <h3><a href={repo.url.clone()}>{&repo.name}</a></h3>
                            <p>{format!("Status: {}", &repo.status)}</p>
                        </li>
                    })}
                </ul>
            }
        </div>
    }
}
