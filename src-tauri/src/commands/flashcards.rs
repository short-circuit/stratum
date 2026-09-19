//! Flashcards and spaced repetition commands.
//!
//! SM-2 algorithm for spaced repetition scheduling.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Persist a page's full block set back to its `.md` file, preserving the
/// existing frontmatter (title/tags/aliases/created/modified/extra) and
/// notifying the auto-commit engine. Mirrors the disk-write contract of
/// `save_blocks` / `toggle_block_marker` so DB-only mutations (SM-2
/// scheduling) survive an app restart, where the boot-time
/// `sync_filesystem_to_db` rebuilds SQLite from disk.
fn write_page_to_disk(
    state: &crate::commands::vault::VaultState,
    store: &pkm_block::BlockStore,
    page_path: &str,
) -> Result<(), String> {
    let all_blocks = store
        .get_blocks_by_page(page_path)
        .map_err(|e| e.to_string())?;
    let body = pkm_markdown::block_parser::serialize_blocks(&all_blocks);
    let full_path = state.vault_path.join(page_path);
    let existing = std::fs::read_to_string(&full_path).unwrap_or_default();
    let markdown = pkm_markdown::block_parser::assemble_blocks_markdown(&existing, &body, None);

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full_path, &markdown).map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlashcardDto {
    pub id: String,
    pub front: String,
    pub back: String,
    pub page_path: String,
    pub ease_factor: f64,
    pub interval_days: u32,
    pub repetitions: u32,
    pub next_review: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewResult {
    pub quality: u8, // 0-5 SM-2 quality rating
}

/// Generate flashcards from blocks with `question::` and `answer::` properties.
#[tauri::command]
pub async fn generate_flashcards(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FlashcardDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    let pages = store.list_pages().map_err(|e| e.to_string())?;

    let mut cards = Vec::new();
    for page_path in &pages {
        let blocks = store
            .get_blocks_by_page(page_path)
            .map_err(|e| e.to_string())?;
        for block in &blocks {
            let question = block.properties.get("question");
            let answer = block.properties.get("answer");
            if let (Some(_q), Some(a)) = (question, answer) {
                let ease_factor = block
                    .properties
                    .get("ease")
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(2.5);
                let interval = block
                    .properties
                    .get("interval")
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0);
                let reps = block
                    .properties
                    .get("reps")
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0);
                let next_review = block
                    .properties
                    .get("next_review")
                    .cloned()
                    .unwrap_or_default();

                cards.push(FlashcardDto {
                    id: block.id.to_string(),
                    // Documented contract (docs/guide/flashcards.md §Card Properties):
                    // the block CONTENT is the question; `.question:: true` is a boolean
                    // marker. Using q.clone() ("true") as the front is wrong.
                    front: block.content.clone(),
                    back: a.clone(),
                    page_path: page_path.clone(),
                    ease_factor,
                    interval_days: interval,
                    repetitions: reps,
                    next_review,
                });
            }
        }
    }

    cards.sort_by(|a, b| {
        let a_due = a.next_review <= chrono::Utc::now().format("%Y-%m-%d").to_string();
        let b_due = b.next_review <= chrono::Utc::now().format("%Y-%m-%d").to_string();
        a_due.cmp(&b_due).reverse()
    });

    Ok(cards)
}

/// Generate cards from a page where each Q&A pair is separated by `---`.
#[tauri::command]
pub async fn generate_cards_from_page(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FlashcardDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let full_path = state.vault_path.join(&page_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;

    let cards = parse_cards_from_markdown(&content, &page_path);
    Ok(cards)
}

/// SM-2 spaced repetition: update card after review.
#[tauri::command]
pub async fn review_card(
    card_id: String,
    quality: u8,
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<FlashcardDto, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let id = Uuid::parse_str(&card_id).map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    let mut block = store.get_block(id).map_err(|e| e.to_string())?;

    // SM-2 algorithm
    let quality = quality.min(5);
    let mut ease = block
        .properties
        .get("ease")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(2.5);
    let mut interval = block
        .properties
        .get("interval")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let mut reps = block
        .properties
        .get("reps")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    if quality >= 3 {
        // Correct response
        reps += 1;
        match reps {
            1 => interval = 1,
            2 => interval = 6,
            _ => interval = ((interval as f64) * ease).round() as u32,
        }
    } else {
        // Incorrect response — reset
        reps = 0;
        interval = 1;
    }

    // Update ease factor
    ease += 0.1 - (5 - quality) as f64 * (0.08 + (5 - quality) as f64 * 0.02);
    if ease < 1.3 {
        ease = 1.3;
    }

    // Calculate next review date
    let next = chrono::Utc::now() + chrono::Duration::days(interval as i64);
    let next_str = next.format("%Y-%m-%d").to_string();

    block.properties.insert("ease".into(), ease.to_string());
    block
        .properties
        .insert("interval".into(), interval.to_string());
    block.properties.insert("reps".into(), reps.to_string());
    block
        .properties
        .insert("next_review".into(), next_str.clone());

    store
        .insert_block(&block, &page_path)
        .map_err(|e| e.to_string())?;

    // Persist the updated schedule to the `.md` file, then keep the search
    // index in sync. Without this, the SM-2 schedule is DB-only and is
    // overwritten on the next app boot by `sync_filesystem_to_db`.
    write_page_to_disk(&state, &store, &page_path)?;
    state.record_change(&page_path);
    let block_index = state.ensure_block_index()?;
    block_index
        .index_block(&block, &page_path)
        .map_err(|e| e.to_string())?;
    block_index.flush().map_err(|e| e.to_string())?;
    drop(state.block_index.take());

    // Keep IndexEngine in sync with the written file
    let vault_path = state.vault_path.clone();
    state
        .ensure_index()?
        .refresh_page(&page_path, &vault_path)
        .map_err(|e| format!("Index refresh failed: {e}"))?;

    Ok(FlashcardDto {
        id: block.id.to_string(),
        // Same contract as generate_flashcards: block content is the question.
        front: block.content.clone(),
        back: block.properties.get("answer").cloned().unwrap_or_default(),
        page_path,
        ease_factor: ease,
        interval_days: interval,
        repetitions: reps,
        next_review: next_str,
    })
}

/// Parse cards from markdown content with Q: and A: markers.
fn parse_cards_from_markdown(content: &str, page_path: &str) -> Vec<FlashcardDto> {
    let lines: Vec<&str> = content.lines().collect();
    let mut cards = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("Q:") || line.starts_with("**Q:**") || line.starts_with("- Q:") {
            let front = line
                .trim_start_matches("- ")
                .trim_start_matches("**Q:** ")
                .trim_start_matches("Q: ")
                .to_string();

            i += 1;
            let mut back_parts = Vec::new();
            while i < lines.len() {
                let next = lines[i].trim();
                if next.starts_with("A:") || next.starts_with("**A:**") || next.starts_with("- A:")
                {
                    let answer = next
                        .trim_start_matches("- ")
                        .trim_start_matches("**A:** ")
                        .trim_start_matches("A: ")
                        .to_string();
                    back_parts.push(answer);
                    break;
                }
                if next.starts_with("Q:") || next.starts_with("**Q:**") || next.starts_with("---") {
                    break;
                }
                i += 1;
            }

            if !back_parts.is_empty() {
                cards.push(FlashcardDto {
                    id: Uuid::new_v4().to_string(),
                    front,
                    back: back_parts.join(" "),
                    page_path: page_path.to_string(),
                    ease_factor: 2.5,
                    interval_days: 0,
                    repetitions: 0,
                    next_review: String::new(),
                });
            }
        }
        i += 1;
    }

    cards
}
