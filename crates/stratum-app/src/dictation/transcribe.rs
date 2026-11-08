//! Transcription pipeline internals for voice dictation.
//!
//! `dictation_transcribe` lives here; progress and completion are streamed
//! through the host's [`EventSink`] under the same event names/payloads the
//! Tauri shell used.

use super::{endpoint_for, load_config, registry};
use crate::dictation::memos::insert_memo_blocks;
use crate::events::EventSink;
use crate::state::{DictationSession, StratumService};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

impl StratumService {
    /// Transcribe a saved clip, enrich it (summary/links/tags) and insert the
    /// memo into `page_path`. Progress is streamed via `dictation:stage` events.
    pub async fn dictation_transcribe(
        &self,
        recording_path: String,
        page_path: String,
        opts: Option<DictationOptsDto>,
        sink: &dyn EventSink,
    ) -> Result<DictationResultDto, String> {
        let (config, vault_path, index_path, store, _registry_path, registry) = {
            let state = self.lock()?;
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

        let emit_sink = sink.shared();
        let emit = move |stage: pkm_dictation::Stage| {
            let name = match stage {
                pkm_dictation::Stage::Transcribing => "transcribing",
                pkm_dictation::Stage::Diarizing => "diarizing",
                pkm_dictation::Stage::Identifying => "identifying",
                pkm_dictation::Stage::Summarizing => "summarizing",
                pkm_dictation::Stage::Linking => "linking",
            };
            emit_sink.emit("dictation:stage", serde_json::json!({ "stage": name }));
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
        let mut state = self.lock()?;
        let block_ids = insert_memo_blocks(&mut state, &page_path, &out.markdown)?;

        // Keep a session so speakers can be renamed/enrolled afterwards.
        state.dictation_sessions.insert(
            recording_path.clone(),
            DictationSession {
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

        sink.emit(
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
}
