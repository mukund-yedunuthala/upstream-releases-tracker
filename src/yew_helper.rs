use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use tracker_libs::RepoData;
use wasm_bindgen::prelude::*;
use web_sys::console;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_without_args(cmd: &str) -> JsValue;
}

pub async fn call_get_repos() -> Result<HashMap<String, RepoData>, String> {
    let js_value = invoke_without_args("get_repos").await;

    match from_value::<HashMap<String, RepoData>>(js_value) {
        Ok(result) => Ok(result),
        Err(e) => {
            let err_msg = format!("Failed to deserialize result: {:?}", e);
            console::error_1(&err_msg.clone().into());
            Err(err_msg)
        }
    }
}
