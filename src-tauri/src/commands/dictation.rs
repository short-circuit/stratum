//! Voice dictation commands: capture, transcribe, speakers, test connection.
//!
//! Thin glue over `pkm-audio`, `pkm-stt` and `pkm-dictation`; the pipeline
//! logic lives in the crates. Inserting the rendered memo follows the same
//! atomic-write flow as `save_blocks`.

use crate::commands::vault::{AppState, VaultState};
use pkm_core::Config;
use pkm_dictation::speakers::{SpeakerEntry, SpeakerRegistry};
use pkm_stt::SttEndpoint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::Emitter;
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
// Transcription pipeline
// ---------------------------------------------------------------------------

/// Transcribe a saved clip, enrich it (summary/links/tags) and insert the
/// memo into `page_path`. Progress is streamed via `dictation:stage` events.
#[tauri::command]
pub async fn dictation_transcribe(
    recording_path: String,
    page_path: String,
    opts: Option<DictationOptsDto>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<DictationResultDto, String> {
    let (config, vault_path, index_path, store, _registry_path, registry) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let config = load_config(&state)?;
        endpoint_for(&config)?; // early validation
        let store = state.get_store().map_err(|e| e.to_string())?;
        let index_path = state.vault_path.join(".pkm").join("search");
        let (registry_path, registry) = registry(&state);
        (
            config,
            state.vault_path.clone(),
            index_path,
            store,
            registry_path,
            registry,
        )
    };

    let opts = opts.unwrap_or_default();
    let clip = PathBuf::from(&recording_path);
    let duration = pkm_audio::decode_flac(&clip)
        .map(|(s, rate)| s.len() as f64 / rate as f64)
        .unwrap_or(0.0);

    let provider = pkm_ai::provider::ProviderFactory::create(&config.ai)
        .map_err(|e| format!("AI provider error: {e}"))?;

    let emit_app = app.clone();
    let emit = move |stage: pkm_dictation::Stage| {
        let name = match stage {
            pkm_dictation::Stage::Transcribing => "transcribing",
            pkm_dictation::Stage::Diarizing => "diarizing",
            pkm_dictation::Stage::Identifying => "identifying",
            pkm_dictation::Stage::Summarizing => "summarizing",
            pkm_dictation::Stage::Linking => "linking",
        };
        let _ = emit_app.emit("dictation:stage", serde_json::json!({ "stage": name }));
    };

    let slug = Path::new(&page_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("voice-memo");
    let clip_rel = clip
        .strip_prefix(&vault_path)
        .unwrap_or(&clip)
        .to_string_lossy()
        .to_string();

    let pipeline = pkm_dictation::Pipeline {
        endpoint: endpoint_for(&config)?,
        store,
        vault_path: &vault_path,
        index_path: &index_path,
        llm: provider.as_ref(),
        llm_model: &config.ai.model,
        transcribe_model: &config.stt.model,
        diarize_model: &config.stt.diarize_model,
        language: config.stt.language.as_deref(),
        registry: &registry,
        on_stage: Some(&emit),
    };
    let popts = pkm_dictation::PipelineOptions {
        clip_path: &clip,
        clip_rel_path: &clip_rel,
        page_slug: slug,
        recorded_at: chrono::Local::now(),
        duration_secs: duration,
        summarize: opts.summarize.unwrap_or(config.stt.auto_summarize),
        diarize: opts.diarize.unwrap_or(config.stt.diarize),
        identify: opts.identify.unwrap_or(config.stt.auto_identify),
    };

    let out = pkm_dictation::run(pipeline, &popts)
        .await
        .map_err(|e| format!("Dictation failed: {e}"))?;

    // Insert memo blocks at the end of the page (same flow as save_blocks).
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let block_ids = insert_memo_blocks(&mut state, &page_path, &out.markdown)?;

    // Keep a session so speakers can be renamed/enrolled afterwards.
    state.dictation_sessions.insert(
        recording_path.clone(),
        super::vault::DictationSession {
            page_path: page_path.clone(),
            clip_path: clip,
            clip_rel_path: clip_rel.clone(),
            recorded_at: chrono::Local::now(),
            duration_secs: duration,
            turns: out.turns.clone(),
            speaker_names: out.speaker_names.clone(),
            summary: out.summary.clone(),
            related: out.related.clone(),
            tags: out.tags.clone(),
            inserted_block_ids: block_ids.clone(),
        },
    );

    let _ = app.emit(
        "dictation:done",
        serde_json::json!({ "inserted": block_ids.len(), "diarized": out.diarized }),
    );

    Ok(DictationResultDto {
        markdown: out.markdown,
        inserted_block_ids: block_ids,
        turns: out
            .turns
            .into_iter()
            .map(|t| SpeakerTurnDto {
                speaker: t.speaker,
                start: t.start,
                end: t.end,
                text: t.text,
            })
            .collect(),
        speaker_names: out.speaker_names,
        num_speakers: out.num_speakers,
        diarized: out.diarized,
        summary: out.summary,
        related: out.related,
        tags: out.tags,
        clip_rel_path: clip_rel,
        duration_secs: duration,
    })
}

// ---------------------------------------------------------------------------
// Speaker assignment (manual names + voice enrollment)
// ---------------------------------------------------------------------------

/// List enrolled voices.
#[tauri::command]
pub fn speaker_list(state: tauri::State<'_, AppState>) -> Result<Vec<SpeakerDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let (_, reg) = registry(&state);
    Ok(reg
        .speakers
        .iter()
        .map(|s| SpeakerDto {
            name: s.name.clone(),
            clip: s.clip.clone(),
            enrolled_at: s.enrolled_at.clone(),
        })
        .collect())
}

/// Assign a name to a speaker of a finished dictation.
///
/// With `enroll=true` the speaker's voice is embedded and stored, so future
/// recordings auto-identify them. The memo in the note is re-rendered with
/// the new name.
#[tauri::command]
pub async fn speaker_assign(
    recording_path: String,
    speaker_id: String,
    name: String,
    enroll: bool,
    state: tauri::State<'_, AppState>,
) -> Result<SpeakerAssignDto, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Name must not be empty".into());
    }

    // Phase 1 (no lock held during await): load session + config snapshot.
    let (session, registry_path, endpoint) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let session = state
            .dictation_sessions
            .get(&recording_path)
            .cloned()
            .ok_or_else(|| "No dictation session for this recording".to_string())?;
        let (registry_path, _) = registry(&state);
        let config = load_config(&state)?;
        (session, registry_path, endpoint_for(&config)?)
    };

    // Find this speaker's first turn for the enrollment slice.
    let turn = session
        .turns
        .iter()
        .find(|t| t.speaker.as_deref() == Some(speaker_id.as_str()));

    let mut entry = SpeakerEntry {
        name: name.clone(),
        clip: None,
        embedding: None,
        enrolled_at: None,
    };

    let mut enrolled = false;
    if enroll {
        if let Some(turn) = turn {
            let (samples, rate) = pkm_audio::decode_flac(&session.clip_path)
                .map_err(|e| format!("Failed to read recording: {e}"))?;
            let rate_f = rate as f64;
            let start = (turn.start * rate_f).round() as usize;
            let len = ((turn.end - turn.start).min(4.0) * rate_f).round() as usize;
            if start < samples.len() && len >= (rate as usize) / 2 {
                let slice: Vec<f32> = samples[start..(start + len).min(samples.len())].to_vec();
                // Clip lives at <vault>/assets/recordings/<file>.
                let vault = session
                    .clip_path
                    .parent()
                    .and_then(|p| p.parent())
                    .ok_or_else(|| "Cannot resolve vault path".to_string())?;
                let speakers_dir = vault.join("assets").join("speakers");
                std::fs::create_dir_all(&speakers_dir).map_err(|e| e.to_string())?;
                let clip_rel = format!(
                    "assets/speakers/{}_{}.wav",
                    pkm_audio::sanitize_slug(&name),
                    chrono::Local::now().format("%Y%m%d_%H%M%S")
                );
                let clip_abs = vault.join(&clip_rel);
                pkm_audio::encode_wav_slice(&slice, rate, &clip_abs)
                    .map_err(|e| format!("Failed to save voice sample: {e}"))?;

                let voice = pkm_stt::VoiceIdClient::new(endpoint.clone(), "speechbrain-ecapa-tdnn");
                let embedding = voice
                    .embed(&clip_abs)
                    .await
                    .map_err(|e| format!("Voice enrollment failed: {e}"))?;
                entry.clip = Some(clip_rel);
                entry.embedding = Some(embedding);
                entry.enrolled_at = Some(chrono::Utc::now().to_rfc3339());
                enrolled = true;
            }
        }
        if !enrolled {
            return Err("No usable speech sample for this speaker".into());
        }
    }

    // Phase 2: persist registry + re-render memo + replace blocks.
    let mut state = state.lock().map_err(|e| e.to_string())?;
    {
        let (_, mut reg) = registry(&state);
        reg.upsert(entry);
        reg.save(&registry_path).map_err(|e| e.to_string())?;
    }

    // Re-render the memo with the assigned name and replace the blocks.
    let mut names = session.speaker_names.clone();
    names.insert(speaker_id.clone(), name.clone());
    let meta = pkm_dictation::MemoMeta {
        page_slug: Path::new(&session.page_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("voice-memo"),
        recorded_at: session.recorded_at,
        clip_rel_path: &session.clip_rel_path,
        duration_secs: session.duration_secs,
        speakers: session.turns.iter().filter(|t| t.speaker.is_some()).count(),
    };
    let markdown = pkm_dictation::render_memo(
        &meta,
        &session.turns,
        &names,
        session.summary.as_deref(),
        &session.related,
        &session.tags,
    );

    let new_ids = replace_memo_blocks(&mut state, &session, &markdown)?;
    if let Some(s) = state.dictation_sessions.get_mut(&recording_path) {
        s.speaker_names = names.clone();
        s.inserted_block_ids = new_ids.clone();
    }

    Ok(SpeakerAssignDto {
        name,
        enrolled,
        markdown,
        speaker_names: names,
        inserted_block_ids: new_ids,
    })
}

/// Remove a voice from the registry.
#[tauri::command]
pub fn speaker_delete(name: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let (registry_path, mut reg) = registry(&state);
    reg.remove(&name);
    reg.save(&registry_path).map_err(|e| e.to_string())?;
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

// ---------------------------------------------------------------------------
// Block insertion helpers (mirror save_blocks' atomic flow)
// ---------------------------------------------------------------------------

/// Parse memo markdown into blocks and append them to the page.
fn insert_memo_blocks(
    state: &mut VaultState,
    page_path: &str,
    markdown: &str,
) -> Result<Vec<String>, String> {
    let store = state.get_store().map_err(|e| e.to_string())?;
    let blocks = parse_blocks(markdown)?;

    // Anchor: the current last block of the page (if any).
    let existing = store
        .get_blocks_by_page(page_path)
        .map_err(|e| e.to_string())?;
    let anchor = existing.last().map(|b| b.id);

    store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        let mut prev = anchor;
        for mut block in blocks {
            block.left_id = prev;
            let id = block.id;
            store
                .insert_block(&block, page_path)
                .map_err(|e| e.to_string())?;
            ids.push(id.to_string());
            prev = Some(id);
        }
        Ok(ids)
    })();
    let ids = match result {
        Ok(ids) => {
            store.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            ids
        }
        Err(e) => {
            store.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    };
    refresh_page_index(state, page_path)?;
    Ok(ids)
}

/// Replace a memo's blocks (after rename) with re-rendered ones.
fn replace_memo_blocks(
    state: &mut VaultState,
    session: &super::vault::DictationSession,
    markdown: &str,
) -> Result<Vec<String>, String> {
    let store = state.get_store().map_err(|e| e.to_string())?;

    // Anchor = the block before the first memo block (its left_id).
    let first_id = session
        .inserted_block_ids
        .first()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let anchor = first_id
        .and_then(|id| store.get_block(id).ok())
        .and_then(|b| b.left_id);

    store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<Vec<String>, String> {
        for id_str in &session.inserted_block_ids {
            if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                store.delete_block(id).map_err(|e| e.to_string())?;
            }
        }
        let mut ids = Vec::new();
        let mut prev = anchor;
        for mut block in parse_blocks(markdown)? {
            block.left_id = prev;
            let id = block.id;
            store
                .insert_block(&block, &session.page_path)
                .map_err(|e| e.to_string())?;
            ids.push(id.to_string());
            prev = Some(id);
        }
        Ok(ids)
    })();
    let ids = match result {
        Ok(ids) => {
            store.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            ids
        }
        Err(e) => {
            store.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    };
    refresh_page_index(state, &session.page_path)?;
    Ok(ids)
}

/// Parse markdown into fresh blocks (block IDs regenerated on parse).
fn parse_blocks(markdown: &str) -> Result<Vec<pkm_block::Block>, String> {
    let (_, _, blocks) = pkm_markdown::block_parser::parse_document(markdown);
    Ok(blocks)
}

/// Reindex a page after block changes (same as save_blocks tail).
fn refresh_page_index(state: &mut VaultState, page_path: &str) -> Result<(), String> {
    state.record_change(page_path);
    let store = state.get_store().map_err(|e| e.to_string())?;
    let blocks = store
        .get_blocks_by_page(page_path)
        .map_err(|e| e.to_string())?;
    let block_index = state.ensure_block_index()?;
    for block in blocks {
        block_index
            .index_block(&block, page_path)
            .map_err(|e| e.to_string())?;
    }
    block_index.flush().map_err(|e| e.to_string())?;
    drop(state.block_index.take());
    let vault_path = state.vault_path.clone();
    state
        .ensure_index()?
        .refresh_page(page_path, &vault_path)
        .map_err(|e| format!("Index refresh failed: {e}"))?;
    crate::commands::graph::invalidate_graph_cache();
    Ok(())
}
