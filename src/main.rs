use crate::components::repo_form::RepoForm;
use crate::components::repo_list::RepoList;
use yew::prelude::*;

mod components;
mod models;

use models::Repo;

#[function_component(App)]
fn app() -> Html {
    let repos = use_state(|| Vec::<Repo>::new());

    let fetch_repos = {
        let repos = repos.clone();
        Callback::from(move |_| {
            let new_repos = vec![
                Repo {
                    id: 1,
                    name: "Placeholder Repo 1".to_string(),
                    url: "https://github.com/placeholder/repo1".to_string(),
                    status: "Active".to_string(),
                },
                Repo {
                    id: 2,
                    name: "Placeholder Repo 2".to_string(),
                    url: "https://github.com/placeholder/repo2".to_string(),
                    status: "Inactive".to_string(),
                },
            ];
            repos.set(new_repos);
        })
    };

    html! {
        <div class="app">
            <h1>{"Repository Manager"}</h1>
            <RepoForm on_submit={fetch_repos} />
            <RepoList repos={(*repos).clone()} />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
