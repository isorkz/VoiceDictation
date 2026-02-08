use tauri::AppHandle;

pub fn init(_app: &AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        return windows::init(_app);
    }
    #[cfg(not(windows))]
    {
        return Ok(());
    }
}

#[cfg(windows)]
mod windows;

