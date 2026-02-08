use crate::config;
use reqwest::multipart;
use std::path::Path;

#[derive(Debug, serde::Deserialize)]
struct TranscriptionResponse {
    text: String,
}

pub async fn transcribe_wav(path: &Path, cfg: &config::Config) -> Result<String, String> {
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .map_err(|_| "AZURE_OPENAI_API_KEY is not set".to_string())?;
    if api_key.trim().is_empty() {
        return Err("AZURE_OPENAI_API_KEY is empty".to_string());
    }

    let endpoint = cfg.azure.endpoint.trim().trim_end_matches('/');
    if endpoint.is_empty() {
        return Err("Azure endpoint is empty".to_string());
    }
    let deployment = cfg.azure.deployment.trim();
    if deployment.is_empty() {
        return Err("Azure deployment is empty".to_string());
    }
    let api_version = cfg.azure.api_version.trim();
    if api_version.is_empty() {
        return Err("Azure apiVersion is empty".to_string());
    }

    let url = format!(
        "{endpoint}/openai/deployments/{deployment}/audio/transcriptions?api-version={api_version}"
    );

    let wav_bytes = std::fs::read(path)
        .map_err(|e| format!("failed to read wav {}: {e}", path.display()))?;

    let file_part = multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("failed to create multipart part: {e}"))?;

    let form = multipart::Form::new().part("file", file_part);

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("api-key", api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("transcription request failed: {e}"))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("failed to read transcription response: {e}"))?;

    if !status.is_success() {
        return Err(format!("transcription failed ({status}): {body}"));
    }

    let parsed: TranscriptionResponse =
        serde_json::from_str(&body).map_err(|e| format!("failed to parse response json: {e}"))?;

    Ok(parsed.text)
}

