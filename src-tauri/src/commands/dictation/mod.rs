//! Voice dictation commands: capture, transcribe, speakers, test connection.
//!
//! Thin glue over `pkm-audio`, `pkm-stt` and `pkm-dictation`; the pipeline
//! logic lives in the crates. Inserting the rendered memo follows the same
//! atomic-write flow as `save_blocks`.

mod memos;
mod speakers;
mod transcribe;

pub use memos::*;
pub use speakers::*;
pub use transcribe::*;

use crate::commands::vault::{AppState, VaultState};
use pkm_core::Config;
use pkm_dictation::speakers::SpeakerRegistry;
use pkm_stt::SttEndpoint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DictationStartDto {
    pub recording_path: String,
    pub device_name: String,
    pub sample_rate: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DictationStopDto {
    pub recording_path: String,
    pub duration_secs: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct DictationOptsDto {
    pub summarize: Option<bool>,
    pub diarize: Option<bool>,
    pub identify: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeakerTurnDto {
    pub speaker: Option<String>,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DictationResultDto {
    pub markdown: String,
    pub inserted_block_ids: Vec<String>,
    pub turns: Vec<SpeakerTurnDto>,
    pub speaker_names: HashMap<String, String>,
    pub num_speakers: usize,
    pub diarized: bool,
    pub summary: Option<String>,
    pub related: Vec<String>,
    pub tags: Vec<String>,
    pub clip_rel_path: String,
    pub duration_secs: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SttTestDto {
    pub ok: bool,
    pub models: Vec<String>,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeakerDto {
    pub name: String,
    pub clip: Option<String>,
    pub enrolled_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeakerAssignDto {
    pub name: String,
    pub enrolled: bool,
    pub markdown: String,
    pub speaker_names: HashMap<String, String>,
    pub inserted_block_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_config(state: &VaultState) -> Result<Config, String> {
    let config_path = state.vault_path.join(".pkm").join("config.toml");
    if !config_path.exists() {
        return Err("STT not configured. Set the transcription endpoint in Settings.".into());
    }
    Config::load(&config_path).map_err(|e| e.to_string())
}

fn endpoint_for(config: &Config) -> Result<SttEndpoint, String> {
    if config.stt.endpoint.trim().is_empty() {
        return Err("STT not configured. Set the transcription endpoint in Settings.".into());
    }
    SttEndpoint::new(config.stt.endpoint.clone(), config.stt.api_key.clone())
        .map_err(|e| format!("Invalid STT endpoint: {e}"))
}

/// Recordings directory (absolute) from the vault layout config.
fn recordings_dir(state: &VaultState) -> PathBuf {
    let rel = load_config(state)
        .map(|c| c.layout.recordings_dir)
        .unwrap_or_else(|_| "assets/recordings".to_string());
    state.vault_path.join(rel)
}

/// Registry file path + load.
fn registry(state: &VaultState) -> (PathBuf, SpeakerRegistry) {
    let path = state.vault_path.join(".pkm").join("speakers.toml");
    let reg = SpeakerRegistry::load(&path).unwrap_or_default();
    (path, reg)
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// Start a recording destined for `page_path`.
#[tauri::command]
pub fn dictation_start(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<DictationStartDto, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.recorder.is_some() {
        return Err("A recording is already in progress".into());
    }
    let device = pkm_audio::AudioRecorder::default_input()
        .map_err(|e| format!("No microphone available: {e}"))?;
    let handle = pkm_audio::AudioRecorder
        .start()
        .map_err(|e| format!("Failed to start recording: {e}"))?;

    let dir = recordings_dir(&state);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let slug = Path::new(&page_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("voice-memo");
    let recording_path = pkm_audio::recording_path(&dir, slug, chrono::Local::now());

    state.recorder = Some(super::vault::ActiveRecording {
        handle,
        recording_path: recording_path.clone(),
    });
    info!(
        "dictation started device={} path={}",
        device.name,
        recording_path.display()
    );
    Ok(DictationStartDto {
        recording_path: recording_path.to_string_lossy().to_string(),
        device_name: device.name,
        sample_rate: device.sample_rate,
    })
}

/// Stop the recording and save the clip as FLAC (atomic temp+rename).
#[tauri::command]
pub fn dictation_stop(state: tauri::State<'_, AppState>) -> Result<DictationStopDto, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let active = state
        .recorder
        .take()
        .ok_or_else(|| "No recording in progress".to_string())?;
    let result = pkm_audio::AudioRecorder.stop(active.handle);

    let tmp = active.recording_path.with_extension("flac.tmp");
    pkm_audio::encode_flac(&result.samples, result.sample_rate, &tmp)
        .map_err(|e| format!("Failed to encode recording: {e}"))?;
    std::fs::rename(&tmp, &active.recording_path).map_err(|e| e.to_string())?;

    info!(
        "dictation stopped path={} duration={:.1}s",
        active.recording_path.display(),
        result.duration_secs
    );
    Ok(DictationStopDto {
        recording_path: active.recording_path.to_string_lossy().to_string(),
        duration_secs: result.duration_secs,
    })
}

/// Cancel the recording: stop and discard the partial clip.
#[tauri::command]
pub fn dictation_cancel(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let active = state
        .recorder
        .take()
        .ok_or_else(|| "No recording in progress".to_string())?;
    let _ = pkm_audio::AudioRecorder.stop(active.handle);
    std::fs::remove_file(&active.recording_path).ok();
    std::fs::remove_file(active.recording_path.with_extension("flac.tmp")).ok();
    info!("dictation cancelled");
    Ok(())
}

// ---------------------------------------------------------------------------
// Connection test
// ---------------------------------------------------------------------------

/// Probe `GET {endpoint}/v1/models` for the Settings page.
#[tauri::command]
pub async fn stt_test_connection(state: tauri::State<'_, AppState>) -> Result<SttTestDto, String> {
    let endpoint = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let config = load_config(&state)?;
        endpoint_for(&config)?
    };
    let started = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client.get(format!("{}/v1/models", endpoint.base_url));
    if let Some(auth) = endpoint.api_key.as_ref() {
        req = req.header("Authorization", format!("Bearer {auth}"));
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            #[derive(serde::Deserialize)]
            struct Models {
                #[serde(default)]
                data: Vec<Model>,
            }
            #[derive(serde::Deserialize)]
            struct Model {
                #[serde(default)]
                id: String,
            }
            let models: Models = resp.json().await.unwrap_or(Models { data: vec![] });
            Ok(SttTestDto {
                ok: true,
                models: models.data.into_iter().map(|m| m.id).collect(),
                latency_ms: started.elapsed().as_millis() as u64,
                error: None,
            })
        }
        Ok(resp) => Err(format!("Endpoint responded with HTTP {}", resp.status())),
        Err(e) => Err(format!("Connection failed: {e}")),
    }
}
