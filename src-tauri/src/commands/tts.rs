//! Tauri adapters for text-to-speech commands.
//!
//! Thin delegation to [`stratum_app::tts`]; synthesis lives in `pkm-ai`.

pub use stratum_app::tts::TtsResultDto;

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn tts_synthesize(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<TtsResultDto, String> {
    state.tts_synthesize(text).await
}

#[tauri::command]
pub async fn tts_speak(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<TtsResultDto, String> {
    state.tts_speak(text).await
}
