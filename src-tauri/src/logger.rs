use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::Manager;

const ERROR_LOG_FILENAME: &str = "errors.log";

pub fn ensure_log_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("failed to resolve app log dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create log dir {}: {e}", dir.display()))?;
    Ok(dir)
}

pub fn append_error(app: &tauri::AppHandle, context: &str, message: &str) -> Result<(), String> {
    let dir = ensure_log_dir(app)?;
    let path = dir.join(ERROR_LOG_FILENAME);

    let ts = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let context = context.replace(['\r', '\n'], " ");
    let message = message.replace('\r', "\\r").replace('\n', "\\n");

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("failed to open log file {}: {e}", path.display()))?;

    writeln!(file, "{ts} [error] {context}: {message}")
        .map_err(|e| format!("failed to write log file {}: {e}", path.display()))?;
    Ok(())
}
