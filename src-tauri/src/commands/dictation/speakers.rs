//! Speaker registry commands: list, assign (with voice enrollment), delete.
//!
//! Commands are re-exported from the parent `dictation` module so
//! `commands::dictation::speaker_*` stays reachable.

use super::{
    endpoint_for, load_config, registry, replace_memo_blocks, SpeakerAssignDto, SpeakerDto,
};
use crate::commands::vault::AppState;
use pkm_dictation::speakers::SpeakerEntry;
use std::path::Path;
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
