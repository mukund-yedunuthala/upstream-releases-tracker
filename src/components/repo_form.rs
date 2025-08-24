use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct RepoFormProps {
    pub on_submit: Callback<()>,
}

#[function_component(RepoForm)]
pub fn repo_form(props: &RepoFormProps) -> Html {
    let onsubmit = {
        let on_submit = props.on_submit.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            on_submit.emit(());
        })
    };

    html! {
        <form onsubmit={onsubmit} class="repo-form">
            <button type="submit">{"Fetch Repositories"}</button>
        </form>
    }
}
