mod config;
mod audio;
mod azure_transcribe;
mod insert;
mod app_state;
mod tray;
mod key_listener;

use serde::Serialize;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt as _;
use std::time::Duration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeyStatus {
    present: bool,
}

#[tauri::command]
fn get_status(state: tauri::State<'_, Mutex<app_state::RuntimeState>>) -> app_state::Status {
    let state = state.lock().expect("state mutex poisoned");
    state.status.clone()
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
fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
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

#[tauri::command]
async fn toggle_recording(
    app: tauri::AppHandle,
) -> Result<(), String> {
    toggle_recording_impl(app).await
}

pub(crate) async fn toggle_recording_impl(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Mutex<app_state::RuntimeState>>();
    let should_stop = {
        let s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        if s.status.state == "Transcribing" || s.status.state == "Inserting" {
            return Err("Busy".to_string());
        }
        s.status.state == "Recording"
    };

    if should_stop {
        return stop_recording_impl(app).await;
    }

    let cfg = config::load_or_default(&app)?;
    let max_seconds = cfg.recording.max_seconds.max(1);

    let tmp = std::env::temp_dir().join(format!(
        "voicedictation-{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("time error: {e}"))?
            .as_millis()
    ));

    let handle = audio::start_recording(tmp.clone())?;
    let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
    s.recording = Some(handle);
    s.recording_path = Some(tmp);
    s.status.state = "Recording".to_string();
    s.status.last_error = None;
    s.recording_token = s.recording_token.wrapping_add(1);
    let token = s.recording_token;

    let _ = app.emit("status_changed", &s.status);

    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_secs(max_seconds));
        let state = app2.state::<Mutex<app_state::RuntimeState>>();
        let should_stop = state
            .lock()
            .ok()
            .is_some_and(|s| s.status.state == "Recording" && s.recording_token == token);
        if should_stop {
            let _ = tauri::async_runtime::block_on(stop_recording_impl(app2));
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_recording(
    app: tauri::AppHandle,
) -> Result<(), String> {
    stop_recording_impl(app).await
}

pub(crate) async fn stop_recording_impl(app: tauri::AppHandle) -> Result<(), String> {
    let cfg = config::load_or_default(&app)?;

    let state = app.state::<Mutex<app_state::RuntimeState>>();
    let (handle, _wav_path) = {
        let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        if s.status.state != "Recording" {
            return Err("Not recording".to_string());
        }
        s.status.state = "Transcribing".to_string();
        let _ = app.emit("status_changed", &s.status);

        let handle = s
            .recording
            .take()
            .ok_or_else(|| "recording handle missing".to_string())?;
        let wav_path = s
            .recording_path
            .take()
            .ok_or_else(|| "recording path missing".to_string())?;
        (handle, wav_path)
    };

    let wav_path = match tauri::async_runtime::spawn_blocking(move || handle.stop())
        .await
        .map_err(|e| format!("recording stop task failed: {e}"))?
    {
        Ok(path) => path,
        Err(e) => {
            let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
            s.status.state = "Idle".to_string();
            s.status.last_error = Some(e.clone());
            let _ = app.emit("error", &e);
            let _ = app.emit("status_changed", &s.status);
            return Err(e);
        }
    };

    let api_key_missing = std::env::var("AZURE_OPENAI_API_KEY")
        .ok()
        .is_none_or(|value| value.trim().is_empty());
    if api_key_missing {
        let _ = std::fs::remove_file(&wav_path);
        let e = "AZURE_OPENAI_API_KEY is not set".to_string();
        let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        s.status.state = "Idle".to_string();
        s.status.last_error = Some(e.clone());
        let _ = app.emit("error", &e);
        let _ = app.emit("status_changed", &s.status);
        return Err(e);
    }

    let text = match azure_transcribe::transcribe_wav(&wav_path, &cfg).await {
        Ok(t) => t,
        Err(e) => {
            let _ = std::fs::remove_file(&wav_path);
            let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
            s.status.state = "Idle".to_string();
            s.status.last_error = Some(e.clone());
            let _ = app.emit("error", &e);
            let _ = app.emit("status_changed", &s.status);
            return Err(e);
        }
    };

    {
        let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        s.status.state = "Inserting".to_string();
        let _ = app.emit("status_changed", &s.status);
    }

    let restore = cfg.insert.restore_clipboard;
    let text2 = text.clone();
    let insert_result = tauri::async_runtime::spawn_blocking(move || insert::clipboard_paste_restore(&text2, restore))
        .await
        .map_err(|e| format!("insert task failed: {e}"))?;

    if let Err(e) = insert_result {
        let _ = std::fs::remove_file(&wav_path);
        let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        s.status.state = "Idle".to_string();
        s.status.last_error = Some(e.clone());
        let _ = app.emit("error", &e);
        let _ = app.emit("status_changed", &s.status);
        return Err(e);
    }

    let _ = std::fs::remove_file(&wav_path);

    let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
    s.status.state = "Idle".to_string();
    s.status.last_error = None;
    let _ = app.emit("transcript_ready", text.clone());
    let _ = app.emit("status_changed", &s.status);

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(app_state::RuntimeState::new()))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            tray::setup(&app.handle())?;
            key_listener::init(&app.handle()).map_err(|e| {
                tauri::Error::Setup(
                    (Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                        as Box<dyn std::error::Error>)
                        .into(),
                )
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            check_api_key,
            get_config,
            set_config,
            get_autostart_enabled,
            set_autostart_enabled,
            test_transcription,
            toggle_recording,
            stop_recording
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
