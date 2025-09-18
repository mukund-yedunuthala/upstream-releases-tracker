use serde::Serialize;
use serde_wasm_bindgen::from_value;
use serde_wasm_bindgen::to_value;
use std::collections::BTreeMap;
use tracker_libs::RepoData;
use wasm_bindgen::prelude::*;
use web_sys::console;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_without_args(cmd: &str) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

pub async fn call_get_repos() -> Result<BTreeMap<String, RepoData>, String> {
    let js_value = invoke_without_args("get_repos").await;

    match from_value::<BTreeMap<String, RepoData>>(js_value) {
        Ok(result) => Ok(result),
        Err(e) => {
            let err_msg = format!("Failed to deserialize result: {:?}", e);
            console::error_1(&err_msg.clone().into());
            Err(err_msg)
        }
    }
}
#[derive(Serialize)]
struct RepoArgs {
    url: String,
}

pub async fn call_del_repo(url: String) {
    let args = RepoArgs { url };
    let js_args = to_value(&args).unwrap(); // serialize to JsValue
    let _ = invoke("delete_repo", js_args).await;
}

pub async fn call_refresh_repo(url: String) {
    let args = RepoArgs { url };
    let js_args = to_value(&args).unwrap(); // serialize to JsValue
    let _ = invoke("refresh_repo", js_args).await;
}

pub async fn call_add_repo(url: String) {
    let args = RepoArgs { url };
    let js_args = to_value(&args).unwrap(); // serialize to JsValue
    let _ = invoke("add_repo", js_args).await;
}

pub async fn call_mark_as_updated(url: String) {
    let args = RepoArgs { url };
    let js_args = to_value(&args).unwrap(); // serialize to JsValue
    let _ = invoke("mark_as_updated", js_args).await;
}
