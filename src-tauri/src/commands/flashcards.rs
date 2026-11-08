//! Tauri adapters for flashcards and spaced repetition commands.

pub use stratum_app::flashcards::{FlashcardDto, ReviewResult};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn generate_flashcards(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FlashcardDto>, String> {
    state.generate_flashcards().await
}

#[tauri::command]
pub async fn generate_cards_from_page(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FlashcardDto>, String> {
    state.generate_cards_from_page(page_path).await
}

#[tauri::command]
pub async fn review_card(
    card_id: String,
    quality: u8,
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<FlashcardDto, String> {
    state.review_card(card_id, quality, page_path).await
}
