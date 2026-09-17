//! Text-to-speech commands.
//!
//! Thin glue over `pkm_ai::tts::TtsClient`; the synthesis logic lives in the
//! crate. The command resolves the endpoint/credentials from the vault's
//! `.pkm/config.toml` (AI settings + optional TTS override).

use crate::commands::vault::AppState;
use pkm_core::Config;
use serde::Serialize;
use tracing::info;

/// Result of a TTS synthesis call.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TtsResultDto {
    /// Base64-encoded audio bytes.
    pub audio_b64: String,
    /// MIME type of the audio (e.g. `audio/mpeg`, `audio/ogg`).
    pub mime: String,
    /// Size in bytes of the decoded audio.
    pub byte_len: usize,
    /// Model used for synthesis.
    pub model: String,
}

/// Map an output-format name to a MIME type for the audio asset.
fn mime_for(format: &str) -> &'static str {
    match format.to_ascii_lowercase().as_str() {
        "mp3" => "audio/mpeg",
        "opus" => "audio/ogg",
        "ogg" => "audio/ogg",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "pcm" => "audio/L16",
        _ => "application/octet-stream",
    }
}

/// Synthesize speech for `text` from the configured AI endpoint.
///
/// Returns the audio as base64 so it can be played in the WebView without
/// writing to disk. Errors are surfaced with the endpoint's message where
/// available.
#[tauri::command]
pub async fn tts_synthesize(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<TtsResultDto, String> {
    let config = {
        let s = state.lock().map_err(|e| e.to_string())?;
        let config_path = s.vault_path.join(".pkm").join("config.toml");
        if !config_path.exists() {
            return Err("AI not configured. Configure the AI provider in Settings → AI.".into());
        }
        Config::load(&config_path).map_err(|e| e.to_string())?
    };

    let client =
        pkm_ai::tts::TtsClient::from_config(&config.ai, &config.tts).map_err(|e| e.to_string())?;

    info!(
        "tts_synthesize text_len={} model={} endpoint={}",
        text.len(),
        client.model(),
        client.endpoint()
    );

    let bytes = client
        .synthesize(&text)
        .await
        .map_err(|e| format!("TTS failed: {e}"))?;

    use base64::Engine;
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    Ok(TtsResultDto {
        audio_b64,
        mime: mime_for(&config.tts.format).to_string(),
        byte_len: bytes.len(),
        model: client.model().to_string(),
    })
}

/// Read-aloud entry point: synthesize `text` from the configured TTS endpoint
/// and return the audio as base64 so the frontend can play it. Honors the
/// voice / format / speed settings from the vault config.
#[tauri::command]
pub async fn tts_speak(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<TtsResultDto, String> {
    let config = {
        let s = state.lock().map_err(|e| e.to_string())?;
        let config_path = s.vault_path.join(".pkm").join("config.toml");
        if !config_path.exists() {
            return Err("AI not configured. Configure the AI provider in Settings → AI.".into());
        }
        Config::load(&config_path).map_err(|e| e.to_string())?
    };

    let client =
        pkm_ai::tts::TtsClient::from_config(&config.ai, &config.tts).map_err(|e| e.to_string())?;

    info!(
        "tts_speak text_len={} model={} endpoint={}",
        text.len(),
        client.model(),
        client.endpoint()
    );

    let bytes = client
        .synthesize(&text)
        .await
        .map_err(|e| format!("TTS failed: {e}"))?;

    use base64::Engine;
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    Ok(TtsResultDto {
        audio_b64,
        mime: mime_for(&config.tts.format).to_string(),
        byte_len: bytes.len(),
        model: client.model().to_string(),
    })
}
