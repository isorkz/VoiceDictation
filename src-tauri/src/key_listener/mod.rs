use tauri::AppHandle;

#[cfg(target_os = "macos")]
pub fn init(app: &AppHandle) -> Result<(), String> {
    macos::init(app)
}

#[cfg(windows)]
pub fn init(app: &AppHandle) -> Result<(), String> {
    windows::init(app)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn init(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
mod macos;
