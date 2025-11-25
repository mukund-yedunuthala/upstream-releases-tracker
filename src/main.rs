use std::collections::BTreeMap;
use tracker_libs::RepoData;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
mod yew_helper;

#[function_component(App)]
fn app() -> Html {
    let repos = use_state(|| None::<BTreeMap<String, RepoData>>);
    let loading = use_state(|| false);
    let input_url = use_state(|| "".to_string());

    // Async action helpers
    let fetch_repos = {
        let repos = repos.clone();
        let loading = loading.clone();
        Callback::from(move |_| {
            let repos = repos.clone();
            let loading = loading.clone();
            spawn_local(async move {
                loading.set(true);
                match yew_helper::call_get_repos().await {
                    Ok(data) => repos.set(Some(data)),
                    Err(err) => web_sys::console::error_1(&err.into()),
                }
                loading.set(false);
            });
        })
    };

    let on_input_change = {
        let input_url = input_url.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                input_url.set(input.value());
            }
        })
    };

    let add_repo = {
        let input_url = input_url.clone();
        let repos = repos.clone();
        let loading = loading.clone();
        Callback::from(move |_| {
            let url = (*input_url).clone();
            if url.trim().is_empty() {
                return;
            }
            let repos = repos.clone();
            let loading = loading.clone();
            let input_url = input_url.clone();
            spawn_local(async move {
                loading.set(true);
                web_sys::console::log_1(&format!("Add repo URL: {}", url).into());
                yew_helper::call_add_repo(url.clone()).await;
                match yew_helper::call_get_repos().await {
                    Ok(data) => repos.set(Some(data)),
                    Err(err) => web_sys::console::error_1(&err.into()),
                }
                loading.set(false);
                input_url.set(String::new());
            });
        })
    };

    html! {
        <div class="app-container">
            <h1>{ "Upstream Releases Tracker" }</h1>
            <div class="input-row">
                <input
                    type="text"
                    placeholder="Enter repository URL"
                    value={(*input_url).clone()}
                    oninput={on_input_change}
                    class="url-input" />
                <button onclick={add_repo} disabled={*loading} class="btn add-btn">
                    { if *loading { "Adding..." } else { "Add Repo" } }
                </button>
            </div>
            <button onclick={fetch_repos.clone()} disabled={*loading} class="btn refresh-btn">
                { if *loading { "Loading..." } else { "Refresh All" } }
            </button>
            { render_repo_list(&repos, &loading) }
        </div>
    }
}

fn render_repo_list(
    repos: &UseStateHandle<Option<BTreeMap<String, RepoData>>>,
    loading: &UseStateHandle<bool>,
) -> Html {
    if let Some(repo_map) = &**repos {
        html! {
            <div class="repos-list">
                { for repo_map.iter().map(|(repo_key, repo)| render_repo_card(repo_key, repo, repos, loading)) }
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

fn render_repo_card(
    repo_key: &String,
    repo: &RepoData,
    repos: &UseStateHandle<Option<BTreeMap<String, RepoData>>>,
    loading: &UseStateHandle<bool>,
) -> Html {
    let spawn_with_refresh =
        |action: fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>| {
            let repo_url = repo_key.clone();
            let repos = repos.clone();
            let loading = loading.clone();
            Callback::from(move |_| {
                let url = repo_url.clone();
                let repos = repos.clone();
                let loading = loading.clone();
                spawn_local(async move {
                    loading.set(true);
                    action(url.clone()).await;
                    match yew_helper::call_get_repos().await {
                        Ok(data) => repos.set(Some(data)),
                        Err(err) => web_sys::console::error_1(&err.into()),
                    }
                    loading.set(false);
                });
            })
        };

    html! {
        <div class="repo-card">
            <div class="repo-info">
                <strong>{ &repo.repo_name }</strong>
                <p class="owner">{ format!("Owner: {}", &repo.owner) }</p>
                <p class="status-indicator">
                    { "Status: " }
                    {
                        if repo.latest_release == repo.system_version {
                            // This block executes if the versions are identical
                            html! { 
                                <span class="up-to-date">
                                    { "Up to Date. | " }
                                    { format!("System: {}", repo.system_version) }
                                </span>
                            }
                        } else {
                            // This block executes if the versions differ
                            html! { 
                                <span class="update-required">
                                    { "Update Required. | " }
                                    { format!("Latest: {} | ", repo.latest_release) }
                                    { format!("System: {}", repo.system_version) }
                                </span>
                            }
                        }
                    }
                </p>
                <a href={repo_key.clone()} target="_blank" rel="noopener noreferrer" class="custom-link-class">
                        { &repo_key }
                </a>
            </div>
            <div class="repo-actions">
            <button
                onclick={spawn_with_refresh(|url| Box::pin(async move {
                    web_sys::console::log_1(&format!("Delete repo: {}", url).into());
                    yew_helper::call_del_repo(url).await;
                }))}
                class="btn delete-btn"
                disabled={**loading}
            >
                { "Delete" }
            </button>
            <button
                onclick={spawn_with_refresh(|url| Box::pin(async move {
                    web_sys::console::log_1(&format!("Refresh repo: {}", url).into());
                    yew_helper::call_refresh_repo(url).await;
                }))}
                class="btn refresh-btn"
                disabled={**loading}
            >
                { "Refresh" }
            </button>
            <button
                onclick={spawn_with_refresh(|url| Box::pin(async move {
                    web_sys::console::log_1(&format!("Mark repo as updated: {}", url).into());
                    yew_helper::call_mark_as_updated(url).await;
                }))}
                class="btn update-btn"
                disabled={**loading}
            >
                { "Mark as Updated" }
            </button>
            </div>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
