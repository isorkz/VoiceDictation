use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub azure: AzureConfig,
    pub hotkey: HotkeyConfig,
    pub thresholds: ThresholdsConfig,
    pub recording: RecordingConfig,
    pub insert: InsertConfig,
    pub sound: SoundConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AzureConfig {
    pub endpoint: String,
    pub deployment: String,
    pub api_version: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HotkeyConfig {
    pub windows: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ThresholdsConfig {
    pub hold_ms: u64,
    pub double_click_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RecordingConfig {
    pub max_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InsertConfig {
    pub restore_clipboard: bool,
    pub postfix: InsertPostfix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SoundConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InsertPostfix {
    None,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            azure: AzureConfig::default(),
            hotkey: HotkeyConfig::default(),
            thresholds: ThresholdsConfig::default(),
            recording: RecordingConfig::default(),
            insert: InsertConfig::default(),
            sound: SoundConfig::default(),
        }
    }
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            deployment: String::new(),
            api_version: "2025-03-01-preview".to_string(),
            api_key: String::new(),
        }
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            windows: "Ctrl".to_string(),
        }
    }
}

impl Default for ThresholdsConfig {
    fn default() -> Self {
        Self {
            hold_ms: 180,
            double_click_ms: 300,
        }
    }
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self { max_seconds: 120 }
    }
}

impl Default for InsertConfig {
    fn default() -> Self {
        Self {
            restore_clipboard: true,
            postfix: InsertPostfix::None,
        }
    }
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub fn load_or_default(app: &tauri::AppHandle) -> Result<Config, String> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read config file {}: {e}", path.display()))?;
    serde_json::from_str::<Config>(&raw)
        .map_err(|e| format!("failed to parse config file {}: {e}", path.display()))
}

pub fn save(app: &tauri::AppHandle, config: &Config) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("failed to serialize config: {e}"))?;

    atomic_write(&path, json.as_bytes())
        .map_err(|e| format!("failed to write config file {}: {e}", path.display()))
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("failed to resolve app config dir: {e}"))?;
    Ok(dir.join("config.json"))
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("failed to create dir {}: {e}", path.display()))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, bytes)?;
    #[cfg(windows)]
    {
        let _ = fs::remove_file(path);
    }
    fs::rename(tmp_path, path)?;
    Ok(())
}
