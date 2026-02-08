mod config;
mod audio;
mod azure_transcribe;

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

#[tauri::command]
async fn test_transcription(app: tauri::AppHandle) -> Result<String, String> {
    if std::env::var("AZURE_OPENAI_API_KEY")
        .ok()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err("AZURE_OPENAI_API_KEY is not set".to_string());
    }

    let cfg = config::load_or_default(&app)?;

    let wav_path = tauri::async_runtime::spawn_blocking(move || {
        let tmp = std::env::temp_dir().join(format!(
            "voicedictation-test-{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| format!("time error: {e}"))?
                .as_millis()
        ));

        let handle = audio::start_recording(tmp.clone())?;
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let path = handle.stop()?;
        Ok::<_, String>(path)
    })
    .await
    .map_err(|e| format!("recording task failed: {e}"))??;

    let text = azure_transcribe::transcribe_wav(&wav_path, &cfg).await?;
    let _ = std::fs::remove_file(&wav_path);
    Ok(text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            check_api_key,
            get_config,
            set_config,
            test_transcription
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
