//! Dictation pipeline: transcribe → diarize → merge → identify speakers →
//! summarize → link/tag → render the memo markdown.
//!
//! Pure crate logic: no Tauri types. The command handler in the Tauri shell
//! wires this into the UI and inserts the result into the current note.

use crate::enrich::{self, parse_wiki_link_lines};
use crate::render::{render_memo, MemoMeta};
use crate::speakers::SpeakerRegistry;
use crate::tags::existing_tags;
use chrono::{DateTime, Local};
use pkm_ai::provider::LlmProvider;
use pkm_block::BlockStore;
use pkm_core::{PkmError, PkmResult};
use pkm_index::related::RelatedFinder;
use pkm_stt::{assign_speakers, identify, Diarizer, SpeakerTurn, SttEndpoint, Transcriber, VoiceIdClient};
use std::collections::HashMap;
use std::path::Path;

/// Minimum cosine similarity for a voice match to be trusted.
pub const VOICE_MATCH_MIN_SCORE: f64 = 0.75;

/// Pipeline stages reported through [`Pipeline::on_stage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Transcribing,
    Diarizing,
    Identifying,
    Summarizing,
    Linking,
}

/// Context for one pipeline run.
pub struct Pipeline<'a> {
    pub endpoint: SttEndpoint,
    pub store: &'a BlockStore,
    pub vault_path: &'a Path,
    pub index_path: &'a Path,
    pub llm: &'a dyn LlmProvider,
    pub llm_model: &'a str,
    pub transcribe_model: &'a str,
    pub diarize_model: &'a str,
    pub language: Option<&'a str>,
    pub registry: &'a SpeakerRegistry,
    pub on_stage: Option<&'a dyn Fn(Stage)>,
}

/// Options for a single dictation run.
pub struct PipelineOptions<'a> {
    pub clip_path: &'a Path,
    pub clip_rel_path: &'a str,
    pub page_slug: &'a str,
    pub recorded_at: DateTime<Local>,
    pub duration_secs: f64,
    pub summarize: bool,
    pub diarize: bool,
    pub identify: bool,
}

/// Result handed back to the command layer.
#[derive(Debug)]
pub struct PipelineOutput {
    pub markdown: String,
    pub turns: Vec<SpeakerTurn>,
    /// speaker id (e.g. "SPEAKER_00") → resolved name.
    pub speaker_names: HashMap<String, String>,
    pub summary: Option<String>,
    pub related: Vec<String>,
    pub tags: Vec<String>,
    pub num_speakers: usize,
    pub diarized: bool,
}

impl Pipeline<'_> {
    fn stage(&self, s: Stage) {
        if let Some(cb) = self.on_stage {
            cb(s);
        }
    }
}

/// Run the full dictation pipeline.
pub async fn run(p: &Pipeline<'_>, opts: &PipelineOptions<'_>) -> PkmResult<PipelineOutput> {
    // 1. Transcribe
    p.stage(Stage::Transcribing);
    let transcriber = Transcriber::new(p.endpoint.clone());
    let transcript = transcriber
        .transcribe(opts.clip_path, p.transcribe_model, p.language)
        .await?;

    // 2. Diarize + merge (optional; degrades to flat transcript)
    let mut diarized = false;
    let turns: Vec<SpeakerTurn> = if opts.diarize {
        p.stage(Stage::Diarizing);
        let diarizer = Diarizer::new(p.endpoint.clone());
        match diarizer.diarize(opts.clip_path, p.diarize_model, p.language).await {
            Ok(diar) => {
                diarized = true;
                assign_speakers(&transcript, &diar)
            }
            Err(PkmError::Unsupported(_)) => assign_speakers(&transcript, &Default::default()),
            Err(e) => return Err(e),
        }
    } else {
        assign_speakers(&transcript, &Default::default())
    };

    let num_speakers = distinct_speakers(&turns).len();

    // 3. Identify speakers against the enrolled registry (optional)
    let mut speaker_names: HashMap<String, String> = HashMap::new();
    if opts.identify && diarized && num_speakers > 0 {
        let enrolled = p.registry.enrollable();
        if !enrolled.is_empty() {
            p.stage(Stage::Identifying);
            let voice = VoiceIdClient::new(p.endpoint.clone(), "speechbrain-ecapa-tdnn");
            for id in distinct_speakers(&turns) {
                if let Some(slice) = speaker_slice(opts.clip_path, &turns, &id)? {
                    if let Ok(embedding) = voice.embed(&slice).await {
                        let matches = identify(&embedding, &enrolled, VOICE_MATCH_MIN_SCORE);
                        if let Some(best) = matches.first() {
                            speaker_names.insert(id.clone(), best.name.clone());
                        }
                    }
                }
            }
        }
    }

    // 4. Transcript text for the LLM steps
    let transcript_text = transcript.full_text();

    // 5. Summary
    let mut summary: Option<String> = None;
    if opts.summarize && !transcript_text.is_empty() {
        p.stage(Stage::Summarizing);
        summary = Some(enrich::summarize(p.llm, p.llm_model, &transcript_text).await?);
    }

    // 6. Related notes + tags (only existing vault content)
    let mut related: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    if !transcript_text.is_empty() {
        p.stage(Stage::Linking);
        let split_pred = |c: char| c.is_whitespace() || c == '#' || c == '*' || c == '[' || c == ']';
        let current_slug = Path::new(opts.page_slug).file_stem().and_then(|s| s.to_str());
        let candidates: Vec<String> = RelatedFinder::new()
            .split_predicate(split_pred)
            .find_related(p.store, p.index_path, &transcript_text, current_slug)
            .ok()
            .unwrap_or_default()
            .into_iter()
            .map(|r| r.title)
            .collect();
        related = enrich::suggest_related(p.llm, p.llm_model, &transcript_text, &candidates).await?;

        let existing = existing_tags(p.store, p.vault_path);
        tags = enrich::suggest_tags(p.llm, p.llm_model, &transcript_text, &existing).await?;
    }

    // 7. Render
    let meta = MemoMeta {
        page_slug: opts.page_slug,
        recorded_at: opts.recorded_at,
        clip_rel_path: opts.clip_rel_path,
        duration_secs: opts.duration_secs,
        speakers: num_speakers,
    };
    let markdown = render_memo(&meta, &turns, &speaker_names, summary.as_deref(), &related, &tags);

    Ok(PipelineOutput {
        markdown,
        turns,
        speaker_names,
        summary,
        related,
        tags,
        num_speakers,
        diarized,
    })
}

/// Distinct speaker ids in order of first appearance.
fn distinct_speakers(turns: &[SpeakerTurn]) -> Vec<String> {
    let mut seen = Vec::new();
    for t in turns {
        if let Some(id) = &t.speaker {
            if !seen.iter().any(|s| s == id) {
                seen.push(id.clone());
            }
        }
    }
    seen
}

/// Extract a short WAV slice of the first turn of a speaker for voice ID.
fn speaker_slice(clip_path: &Path, turns: &[SpeakerTurn], speaker: &str) -> PkmResult<Option<std::path::PathBuf>> {
    const SLICE_SECS: f64 = 4.0;
    let Some(turn) = turns.iter().find(|t| t.speaker.as_deref() == Some(speaker)) else {
        return Ok(None);
    };
    let (samples, rate) = pkm_audio::decode_flac(clip_path)?;
    let rate_f = rate as f64;
    let start = (turn.start * rate_f).round() as usize;
    let len = ((turn.end - turn.start).min(SLICE_SECS) * rate_f).round() as usize;
    if start >= samples.len() || len == 0 {
        return Ok(None);
    }
    let slice: Vec<f32> = samples[start..(start + len).min(samples.len())].to_vec();
    if slice.len() < (rate as usize) / 2 {
        return Ok(None); // < 0.5s of audio is useless for identification
    }

    let dir = clip_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".voice_slices");
    std::fs::create_dir_all(&dir).map_err(PkmError::from)?;
    let out = dir.join(format!("{speaker}.wav"));
    pkm_audio::encode_wav_slice(&slice, rate, &out)?;
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distinct_speakers_order() {
        let turns = vec![
            SpeakerTurn { speaker: Some("SPEAKER_01".into()), start: 0.0, end: 1.0, text: "a".into() },
            SpeakerTurn { speaker: Some("SPEAKER_00".into()), start: 1.0, end: 2.0, text: "b".into() },
            SpeakerTurn { speaker: Some("SPEAKER_01".into()), start: 2.0, end: 3.0, text: "c".into() },
            SpeakerTurn { speaker: None, start: 3.0, end: 4.0, text: "d".into() },
        ];
        assert_eq!(distinct_speakers(&turns), vec!["SPEAKER_01", "SPEAKER_00"]);
    }

    #[test]
    fn test_parse_wiki_link_lines_reexport() {
        assert_eq!(
            parse_wiki_link_lines("[[a]]\n[[b|c]]"),
            vec!["a", "b"]
        );
    }
}
