mod config;
mod audio;
mod azure_transcribe;
mod insert;
mod app_state;
mod tray;
mod key_listener;
mod logger;

use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt as _;
use std::time::Duration;

fn emit_status(app: &tauri::AppHandle, status: &app_state::Status) {
    let _ = app.emit("status_changed", status);
    let _ = tray::update_for_status(app, status);
}

fn report_error(app: &tauri::AppHandle, context: &str, message: &str) {
    let _ = logger::append_error(app, context, message);
    let _ = app.emit("error", message);
}

#[cfg(target_os = "macos")]
type SystemSoundID = u32;

#[cfg(target_os = "macos")]
#[link(name = "AudioToolbox", kind = "framework")]
extern "C" {
    fn AudioServicesCreateSystemSoundID(in_file_url: core_foundation::url::CFURLRef, out_id: *mut SystemSoundID) -> i32;
    fn AudioServicesPlaySystemSound(in_sound_id: SystemSoundID);
}

fn play_start_sound() {
    #[cfg(target_os = "macos")]
    {
        play_macos_system_sound("start");
    }

    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows::Win32::UI::WindowsAndMessaging::MB_OK;
        let _ = MessageBeep(MB_OK);
    }
}

fn play_stop_sound() {
    #[cfg(target_os = "macos")]
    {
        play_macos_system_sound("stop");
    }

    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION;
        let _ = MessageBeep(MB_ICONEXCLAMATION);
    }
}

#[cfg(target_os = "macos")]
fn play_macos_system_sound(kind: &'static str) {
    use core_foundation::base::TCFType as _;
    use core_foundation::url::CFURL;
    use std::path::Path;
    use std::sync::OnceLock;

    static START: OnceLock<Option<SystemSoundID>> = OnceLock::new();
    static STOP: OnceLock<Option<SystemSoundID>> = OnceLock::new();

    fn load(sound_path: &'static str) -> Option<SystemSoundID> {
        let url = CFURL::from_path(Path::new(sound_path), false)?;
        let mut id: SystemSoundID = 0;
        let status = unsafe { AudioServicesCreateSystemSoundID(url.as_concrete_TypeRef(), &mut id) };
        if status == 0 { Some(id) } else { None }
    }

    let id = match kind {
        "start" => *START.get_or_init(|| load("/System/Library/Sounds/Pop.aiff")),
        "stop" => *STOP.get_or_init(|| load("/System/Library/Sounds/Tink.aiff")),
        _ => None,
    };

    if let Some(id) = id {
        unsafe { AudioServicesPlaySystemSound(id) };
    }
}

#[tauri::command]
fn get_status(state: tauri::State<'_, Mutex<app_state::RuntimeState>>) -> app_state::Status {
    let state = state.lock().expect("state mutex poisoned");
    state.status.clone()
}

#[tauri::command]
fn get_config(app: tauri::AppHandle) -> Result<config::Config, String> {
    config::load_or_default(&app).inspect_err(|e| {
        let _ = logger::append_error(&app, "get_config", e);
    })
}

#[tauri::command]
fn set_config(app: tauri::AppHandle, config: config::Config) -> Result<(), String> {
    config::save(&app, &config).inspect_err(|e| {
        let _ = logger::append_error(&app, "set_config", e);
    })
}

#[tauri::command]
fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| e.to_string())
        .inspect_err(|e| {
            let _ = logger::append_error(&app, "get_autostart_enabled", e);
        })
}

#[tauri::command]
fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch()
            .enable()
            .map_err(|e| e.to_string())
            .inspect_err(|e| {
                let _ = logger::append_error(&app, "set_autostart_enabled", e);
            })
    } else {
        app.autolaunch()
            .disable()
            .map_err(|e| e.to_string())
            .inspect_err(|e| {
                let _ = logger::append_error(&app, "set_autostart_enabled", e);
            })
    }
}

#[tauri::command]
async fn test_transcription(app: tauri::AppHandle) -> Result<String, String> {
    let cfg = config::load_or_default(&app).inspect_err(|e| {
        let _ = logger::append_error(&app, "test_transcription", e);
    })?;

    let wav_path = tauri::async_runtime::spawn_blocking(move || {
        let tmp = std::env::temp_dir().join(format!(
            "voicedictation-test-{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| format!("time error: {e}"))?
                .as_millis()
        ));

        let handle = audio::start_recording(tmp.clone())?;
        std::thread::sleep(std::time::Duration::from_millis(3000));
        let path = handle.stop()?;
        Ok::<_, String>(path)
    })
    .await
    .map_err(|e| format!("recording task failed: {e}"))
    .inspect_err(|e| {
        let _ = logger::append_error(&app, "test_transcription", e);
    })??;

    let text = azure_transcribe::transcribe_wav(&wav_path, &cfg)
        .await
        .inspect_err(|e| {
            let _ = logger::append_error(&app, "test_transcription", e);
        })?;
    let _ = std::fs::remove_file(&wav_path);
    Ok(text)
}

#[tauri::command]
async fn toggle_recording(
    app: tauri::AppHandle,
) -> Result<(), String> {
    toggle_recording_impl(app.clone())
        .await
        .inspect_err(|e| {
            let _ = logger::append_error(&app, "toggle_recording", e);
        })
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

    let cfg = config::load_or_default(&app).inspect_err(|e| {
        let _ = logger::append_error(&app, "toggle_recording", e);
    })?;
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

    let status = s.status.clone();
    drop(s);
    emit_status(&app, &status);
    play_start_sound();

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
    let cfg = config::load_or_default(&app).inspect_err(|e| {
        let _ = logger::append_error(&app, "stop_recording", e);
    })?;

    let state = app.state::<Mutex<app_state::RuntimeState>>();
    let (handle, _wav_path, transcribing_status) = {
        let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        if s.status.state != "Recording" {
            let e = "Not recording".to_string();
            let _ = logger::append_error(&app, "stop_recording", &e);
            return Err(e);
        }
        s.status.state = "Transcribing".to_string();
        let status = s.status.clone();

        let handle = s
            .recording
            .take()
            .ok_or_else(|| "recording handle missing".to_string())?;
        let wav_path = s
            .recording_path
            .take()
            .ok_or_else(|| "recording path missing".to_string())?;
        (handle, wav_path, status)
    };
    emit_status(&app, &transcribing_status);
    play_stop_sound();

    let stop_result = tauri::async_runtime::spawn_blocking(move || handle.stop())
        .await
        .map_err(|e| format!("recording stop task failed: {e}"));
    let wav_path = match stop_result {
        Ok(result) => result,
        Err(e) => {
            let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
            s.status.state = "Idle".to_string();
            s.status.last_error = Some(e.clone());
            let status = s.status.clone();
            drop(s);
            report_error(&app, "stop_recording", &e);
            emit_status(&app, &status);
            return Err(e);
        }
    };

    let wav_path = match wav_path {
        Ok(path) => path,
        Err(e) => {
            let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
            s.status.state = "Idle".to_string();
            s.status.last_error = Some(e.clone());
            let status = s.status.clone();
            drop(s);
            report_error(&app, "stop_recording", &e);
            emit_status(&app, &status);
            return Err(e);
        }
    };

    if cfg.azure.api_key.trim().is_empty() {
        let _ = std::fs::remove_file(&wav_path);
        let e = "Azure apiKey is empty".to_string();
        let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        s.status.state = "Idle".to_string();
        s.status.last_error = Some(e.clone());
        let status = s.status.clone();
        drop(s);
        report_error(&app, "stop_recording", &e);
        emit_status(&app, &status);
        return Err(e);
    }

    let text = match azure_transcribe::transcribe_wav(&wav_path, &cfg).await {
        Ok(t) => t,
        Err(e) => {
            let _ = std::fs::remove_file(&wav_path);
            let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
            s.status.state = "Idle".to_string();
            s.status.last_error = Some(e.clone());
            let status = s.status.clone();
            drop(s);
            report_error(&app, "stop_recording", &e);
            emit_status(&app, &status);
            return Err(e);
        }
    };

    let inserting_status = {
        let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        s.status.state = "Inserting".to_string();
        s.status.clone()
    };
    emit_status(&app, &inserting_status);

    let restore = cfg.insert.restore_clipboard;
    let text2 = text.clone();
    let insert_result = tauri::async_runtime::spawn_blocking(move || {
        insert::clipboard_paste_restore(&text2, restore)
    })
    .await
    .map_err(|e| format!("insert task failed: {e}"));

    let insert_result = match insert_result {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_file(&wav_path);
            let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
            s.status.state = "Idle".to_string();
            s.status.last_error = Some(e.clone());
            let status = s.status.clone();
            drop(s);
            report_error(&app, "stop_recording", &e);
            emit_status(&app, &status);
            return Err(e);
        }
    };

    if let Err(e) = insert_result {
        let _ = std::fs::remove_file(&wav_path);
        let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
        s.status.state = "Idle".to_string();
        s.status.last_error = Some(e.clone());
        let status = s.status.clone();
        drop(s);
        report_error(&app, "stop_recording", &e);
        emit_status(&app, &status);
        return Err(e);
    }

    let _ = std::fs::remove_file(&wav_path);

    let mut s = state.lock().map_err(|_| "state mutex poisoned".to_string())?;
    s.status.state = "Idle".to_string();
    s.status.last_error = None;
    let status = s.status.clone();
    let _ = app.emit("transcript_ready", text.clone());
    drop(s);
    emit_status(&app, &status);

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
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                app.set_dock_visibility(false);
            }

            tray::setup(&app.handle())?;
            key_listener::init(&app.handle()).map_err(|e| {
                let _ = logger::append_error(&app.handle(), "setup:key_listener", &e);
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
