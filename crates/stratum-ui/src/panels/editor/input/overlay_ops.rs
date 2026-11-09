//! Slash, wiki-link and marker completion application.
//!
//! Completions are computed by [`stratum_editor`] menus; this module strips the
//! typed token from the block, applies the selected action and keeps the overlay
//! state in sync with the caret.

use gpui_kit::*;
use stratum_editor::menus::{
    filter_slash_items, marker_menu_items, slash_items, slash_row_count, wiki_query, wiki_targets,
    SlashAction, WikiPage,
};

use crate::panels::editor::overlay::{detect_overlay, wiki_row_count, OverlayState};
use crate::panels::editor::EditorView;

/// Number of selectable entries in the active overlay.
pub(crate) fn overlay_len(view: &EditorView) -> usize {
    match &view.overlay {
        OverlayState::Slash { query, .. } => {
            let is_journal = view.path().starts_with("journals/");
            let has_selection = view.state.selection().is_some();
            slash_row_count(
                filter_slash_items(&slash_items(has_selection, is_journal), query).len(),
            )
        }
        OverlayState::Wiki { query, .. } => {
            wiki_row_count(wiki_targets(query, &wiki_pages(view)).len())
        }
        OverlayState::Marker { .. } => marker_menu_items().len(),
        _ => 0,
    }
}

/// Apply the selected entry of the active overlay.
pub(crate) fn apply_overlay(
    view: &mut EditorView,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    match view.overlay.clone() {
        OverlayState::Slash { query, selected } => apply_slash(view, &query, selected, window, cx),
        OverlayState::Wiki { query, selected } => apply_wiki(view, &query, selected, cx),
        OverlayState::Marker { selected } => apply_marker(view, selected, cx),
        OverlayState::DeadLink { target } => apply_dead_link(view, &target, cx),
        _ => {}
    }
}

fn apply_slash(
    view: &mut EditorView,
    query: &str,
    selected: usize,
    _window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    let Some(caret) = view.state.caret() else {
        return;
    };
    let content = view
        .state
        .document()
        .block(caret.block_id)
        .map(|block| block.content.clone())
        .unwrap_or_default();
    let Some(token_start) = slash_token_start(&content, caret.byte_offset) else {
        return;
    };
    let is_journal = view.path().starts_with("journals/");
    let has_selection = view.state.selection().is_some();
    let items = filter_slash_items(&slash_items(has_selection, is_journal), query);
    if items.is_empty() {
        view.overlay = OverlayState::None;
        return;
    }
    let index = selected.min(items.len().saturating_sub(1));
    let Some(item) = items.get(index) else {
        view.overlay = OverlayState::None;
        return;
    };
    let action = SlashAction::ALL
        .iter()
        .copied()
        .find(|action| action.id() == item.id);
    // Capture the selection before `set_caret` clears it: selection-scope AI
    // actions must send the text that was selected when the menu opened.
    let selection_text = view.state.selected_text();
    let stripped = format!(
        "{}{}",
        &content[..token_start],
        &content[caret.byte_offset..]
    );
    let replacement = match action {
        Some(SlashAction::CodeBlock) => "```\n\n```".to_string(),
        Some(SlashAction::Mermaid) => "```mermaid\n\n```".to_string(),
        Some(SlashAction::MathEquation) => "$$\n\n$$".to_string(),
        Some(SlashAction::Quote) => {
            if stripped.starts_with('>') {
                stripped
            } else {
                format!("> {stripped}")
            }
        }
        _ => stripped,
    };
    let replacement_len = replacement.len();
    let _ = view
        .state
        .replace_block_content(caret.block_id, replacement);
    let caret_offset = match action {
        Some(SlashAction::Quote) => token_start.saturating_add(2),
        _ => token_start.min(replacement_len),
    };
    let _ = view.state.set_caret(caret.block_id, caret_offset);
    match action {
        Some(SlashAction::Paragraph) | Some(SlashAction::Quote) => {
            let _ = view.state.set_heading_level(None);
        }
        Some(SlashAction::Heading1) => {
            let _ = view.state.set_heading_level(Some(1));
        }
        Some(SlashAction::Heading2) => {
            let _ = view.state.set_heading_level(Some(2));
        }
        Some(SlashAction::Heading3) => {
            let _ = view.state.set_heading_level(Some(3));
        }
        _ => {}
    }
    if let Some(action) = action {
        let document = view.state.document();
        let page_text = document.plain_text();
        let page_markdown = document.plain_markdown_lossy();
        if let Some(call) = stratum_editor::menus::ai_call_for(
            action,
            selection_text.as_deref(),
            &page_text,
            &page_markdown,
            view.path(),
        ) {
            view.start_ai(call, item.title_key, cx);
        }
    }
    view.overlay = OverlayState::None;
    view.after_edit(cx);
}

fn apply_wiki(view: &mut EditorView, query: &str, selected: usize, cx: &mut Context<EditorView>) {
    let Some(caret) = view.state.caret() else {
        return;
    };
    let content = view
        .state
        .document()
        .block(caret.block_id)
        .map(|block| block.content.clone())
        .unwrap_or_default();
    let Some(wiki) = wiki_query(&content, caret.byte_offset) else {
        view.overlay = OverlayState::None;
        return;
    };
    let targets = wiki_targets(query, &wiki_pages(view));
    let insertion = if targets.is_empty() {
        // Close the open `[[term` so the link is never left unterminated.
        format!("{}]]", wiki.term)
    } else {
        let index = selected.min(targets.len() - 1);
        format!("{}]]", targets[index].target)
    };
    let replacement = format!(
        "{}{}{}",
        &content[..wiki.replace_range.start],
        insertion,
        &content[wiki.replace_range.end..]
    );
    let _ = view
        .state
        .replace_block_content(caret.block_id, replacement);
    let offset = wiki.replace_range.start + insertion.len();
    let _ = view.state.set_caret(caret.block_id, offset);
    view.overlay = OverlayState::None;
    view.after_edit(cx);
}

fn apply_marker(view: &mut EditorView, selected: usize, cx: &mut Context<EditorView>) {
    let Some(caret) = view.state.caret() else {
        return;
    };
    let content = view
        .state
        .document()
        .block(caret.block_id)
        .map(|block| block.content.clone())
        .unwrap_or_default();
    let Some(token_start) = marker_token_start(&content, caret.byte_offset) else {
        return;
    };
    let stripped = format!(
        "{}{}",
        &content[..token_start],
        &content[caret.byte_offset..]
    );
    let _ = view
        .state
        .replace_block_content(caret.block_id, stripped.clone());
    let _ = view
        .state
        .set_caret(caret.block_id, token_start.min(stripped.len()));
    if let Some(item) = marker_menu_items().get(selected) {
        if let Some(marker) = item.marker {
            let _ = view.state.set_marker(Some(marker));
        }
        if let Some(priority) = item.priority {
            let _ = view.state.set_priority(Some(priority));
        }
    }
    view.overlay = OverlayState::None;
    view.after_edit(cx);
}

/// Create the page behind an open dead-link overlay and navigate to it.
///
/// Mirrors `DeadLinkCreatePopup.tsx`: creation failures that report an existing
/// page still navigate to the target path, because the link is valid once the
/// page exists. Any other failure is surfaced through the save chip and the
/// popup closes.
pub(crate) fn apply_dead_link(view: &mut EditorView, target: &str, cx: &mut Context<EditorView>) {
    match view.provider.create_page(target, None) {
        Ok(path) => {
            view.pending_navigation = Some(path);
            view.overlay = OverlayState::None;
            cx.notify();
        }
        Err(error) if error.to_lowercase().contains("already exists") => {
            let path = if target.ends_with(".md") {
                target.to_string()
            } else {
                format!("{target}.md")
            };
            view.pending_navigation = Some(path);
            view.overlay = OverlayState::None;
            cx.notify();
        }
        Err(error) => {
            view.save.fail(error);
            view.overlay = OverlayState::None;
            cx.notify();
        }
    }
}

/// Recent pages as wiki autocomplete candidates.
pub(crate) fn wiki_pages(view: &EditorView) -> Vec<WikiPage> {
    view.provider
        .snapshot()
        .recents
        .into_iter()
        .map(|page| {
            let slug = page
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&page.path)
                .trim_end_matches(".md")
                .to_string();
            WikiPage {
                path: page.path,
                slug,
                title: page.title,
            }
        })
        .collect()
}

/// Start offset of a `/` slash token ending at `caret`.
pub(crate) fn slash_token_start(content: &str, caret: usize) -> Option<usize> {
    let before = &content[..caret.min(content.len())];
    let token = before
        .rsplit_once(char::is_whitespace)
        .map(|(_, token)| token)
        .unwrap_or(before);
    if token.starts_with('/') {
        Some(caret - token.len())
    } else {
        None
    }
}

/// Start offset of a `:` marker token ending at `caret`.
pub(crate) fn marker_token_start(content: &str, caret: usize) -> Option<usize> {
    let before = &content[..caret.min(content.len())];
    let token = before
        .rsplit_once(char::is_whitespace)
        .map(|(_, token)| token)
        .unwrap_or(before);
    if token.starts_with(':') && token.len() > 1 {
        Some(caret - token.len())
    } else {
        None
    }
}

/// Recompute the overlay from the caret position.
pub(crate) fn refresh_overlay(view: &mut EditorView) {
    let Some(caret) = view.state.caret() else {
        view.overlay = OverlayState::None;
        return;
    };
    let Some(content) = view
        .state
        .document()
        .block(caret.block_id)
        .map(|block| block.content.clone())
    else {
        view.overlay = OverlayState::None;
        return;
    };
    let is_journal = view.path().starts_with("journals/");
    let has_selection = view.state.selection().is_some();
    view.overlay = detect_overlay(&content, caret.byte_offset, is_journal, has_selection);
}
