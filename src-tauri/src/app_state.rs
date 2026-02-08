use crate::audio;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub state: String,
    pub last_error: Option<String>,
}

pub struct RuntimeState {
    pub status: Status,
    pub recording: Option<audio::RecordingHandle>,
    pub recording_path: Option<PathBuf>,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            status: Status {
                state: "Idle".to_string(),
                last_error: None,
            },
            recording: None,
            recording_path: None,
        }
    }
}

