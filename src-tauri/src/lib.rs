mod config;
mod audio;

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    state: String,
    last_error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeyStatus {
    present: bool,
}

#[tauri::command]
fn get_status() -> Status {
    Status {
        state: "Idle".to_string(),
        last_error: None,
    }
}

#[tauri::command]
fn check_api_key() -> ApiKeyStatus {
    let present = std::env::var("AZURE_OPENAI_API_KEY")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());

    ApiKeyStatus { present }
}

#[tauri::command]
fn get_config(app: tauri::AppHandle) -> Result<config::Config, String> {
    config::load_or_default(&app)
}

#[tauri::command]
fn set_config(app: tauri::AppHandle, config: config::Config) -> Result<(), String> {
    config::save(&app, &config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            check_api_key,
            get_config,
            set_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
