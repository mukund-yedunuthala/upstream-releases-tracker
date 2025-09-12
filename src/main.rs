use std::collections::HashMap;
use tracker_libs::RepoData;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
mod yew_helper;
use yew_helper::call_get_repos;

#[function_component(App)]
fn app() -> Html {
    let repos = use_state(|| None::<HashMap<String, RepoData>>);
    let loading = use_state(|| false);
    let input_url = use_state(|| "".to_string());

    // Manual fetch repos callback
    let fetch_repos = {
        let repos = repos.clone();
        let loading = loading.clone();
        Callback::from(move |_| {
            let repos = repos.clone();
            let loading = loading.clone();
            spawn_local(async move {
                loading.set(true);
                match call_get_repos().await {
                    Ok(data) => repos.set(Some(data)),
                    Err(err) => web_sys::console::error_1(&err.into()),
                }
                loading.set(false);
            });
        })
    };

    // Handle input change
    let on_input_change = {
        let input_url = input_url.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                input_url.set(input.value());
            }
        })
    };

    // Placeholder for adding a new repo URL command
    let on_add_repo = {
        let input_url = input_url.clone();
        let repos = repos.clone();
        let loading = loading.clone();

        Callback::from(move |_| {
            let url = (*input_url).clone();
            if url.trim().is_empty() {
                // ignore empty
                return;
            }
            let repos = repos.clone();
            let loading = loading.clone();
            spawn_local(async move {
                loading.set(true);
                // TODO: invoke tauri add_repo command with `url`
                web_sys::console::log_1(&format!("Add repo URL: {}", url).into());

                // Clear input after add
                // input_url.set(String::new());

                // Optionally refresh all repos after add
                match call_get_repos().await {
                    Ok(data) => repos.set(Some(data)),
                    Err(err) => web_sys::console::error_1(&err.into()),
                }

                loading.set(false);
            });
        })
    };

    html! {
        html! {
            <div class="app-container">
                <h1>{ "My Repos" }</h1>
                <div class="input-row">
                    <input
                        type="text"
                        placeholder="Enter repository URL"
                        value={(*input_url).clone()}
                        oninput={on_input_change}
                        class="url-input" />
                    <button onclick={on_add_repo} disabled={*loading} class="btn add-btn">
                        { if *loading { "Adding..." } else { "Add Repo" } }
                    </button>
                </div>

                <button onclick={fetch_repos.clone()} disabled={*loading} class="btn refresh-btn">
                    { if *loading { "Loading..." } else { "Refresh All" } }
                </button>

                {
                    if let Some(repo_map) = &*repos {
                        html! {
                            <div class="repos-list">
                                { for repo_map.iter().map(|(repo_key, repo)| {
                                    html! {
                                        <div class="repo-card">
                                            <div class="repo-info">
                                                <strong>{ &repo.repo_name }</strong>
                                                <p class="owner">{ format!("Owner: {}", &repo.owner) }</p>
                                            </div>
                                            <div class="repo-actions">
                                            <button onclick={
                                                let repo_url = repo_key.clone();
                                                Callback::from(move |_| {
                                                    let url = repo_url.clone();
                                                    spawn_local(async move {
                                                        web_sys::console::log_1(&format!("Delete repo: {}", url).into());
                                                        yew_helper::call_del_repo(url).await;
                                                    });
                                                })
                                            } class="btn delete-btn">
                                                { "Delete" }
                                            </button>
                                            <button onclick={
                                                let repo_url = repo_key.clone();
                                                Callback::from(move |_| {
                                                    let url = repo_url.clone();
                                                    spawn_local(async move {
                                                        web_sys::console::log_1(&format!("Refresh repo: {}", url).into());
                                                        yew_helper::call_refresh_repo(url).await;
                                                    });
                                                })
                                                } class="btn refresh-btn">
                                                    { "Refresh" }
                                            </button>
                                            </div>
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    } else {
                        html! {
                            <p class="empty-message">
                                { "No repositories loaded. Use the 'Refresh All' button or add repos above." }
                            </p>
                        }
                    }
                }
            </div>
        }

    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
