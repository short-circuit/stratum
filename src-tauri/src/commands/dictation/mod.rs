//! Tauri adapters for voice-dictation commands.
//!
//! Thin delegation to [`stratum_app::dictation`]; capture, transcription and
//! speaker logic live in the headless crate + `pkm-*` crates.

pub use stratum_app::dictation::{
    DictationOptsDto, DictationResultDto, DictationStartDto, DictationStopDto, MicPermissionDto,
    SpeakerAssignDto, SpeakerDto, SpeakerTurnDto, SttTestDto,
};

use crate::commands::vault::AppState;
use crate::commands::TauriEventSink;

#[tauri::command]
pub fn dictation_start(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<DictationStartDto, String> {
    state.dictation_start(page_path)
}

#[tauri::command]
pub fn dictation_stop(state: tauri::State<'_, AppState>) -> Result<DictationStopDto, String> {
    state.dictation_stop()
}

#[tauri::command]
pub fn dictation_cancel(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.dictation_cancel()
}

#[tauri::command]
pub async fn dictation_transcribe(
    recording_path: String,
    page_path: String,
    opts: Option<DictationOptsDto>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<DictationResultDto, String> {
    state
        .dictation_transcribe(recording_path, page_path, opts, &TauriEventSink(app))
        .await
}

#[tauri::command]
pub fn dictation_insert_clip_ref(
    page_path: String,
    clip_rel_path: String,
    duration_secs: f64,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.dictation_insert_clip_ref(
        page_path,
        clip_rel_path,
        duration_secs,
        &TauriEventSink(app),
    )
}

#[tauri::command]
pub fn mic_permission_status(state: tauri::State<'_, AppState>) -> MicPermissionDto {
    state.mic_permission_status()
}

#[tauri::command]
pub fn request_mic_permission(app: tauri::AppHandle, state: tauri::State<'_, AppState>) {
    state.request_mic_permission(&TauriEventSink(app));
}

#[tauri::command]
pub fn speaker_list(state: tauri::State<'_, AppState>) -> Result<Vec<SpeakerDto>, String> {
    state.speaker_list()
}

#[tauri::command]
pub async fn speaker_assign(
    recording_path: String,
    speaker_id: String,
    name: String,
    enroll: bool,
    state: tauri::State<'_, AppState>,
) -> Result<SpeakerAssignDto, String> {
    state
        .speaker_assign(recording_path, speaker_id, name, enroll)
        .await
}

#[tauri::command]
pub fn speaker_delete(name: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.speaker_delete(name)
}

#[tauri::command]
pub async fn stt_test_connection(state: tauri::State<'_, AppState>) -> Result<SttTestDto, String> {
    state.stt_test_connection().await
}
