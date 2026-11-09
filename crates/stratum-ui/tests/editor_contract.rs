//! Window-free contract tests for the phase-B native editor.
//!
//! These tests exercise the pieces the gpui view delegates to: the deterministic
//! 10 000-block fixture layout, Stage A versus focused-line delimiter visibility,
//! the slash/marker catalogues, save-status transitions and the UTF-16 caret
//! helpers. None of them opens a window, so they run in the same CI job as the
//! rest of the crate.

use stratum_editor::virtualize::MIN_ROW_HEIGHT;
use stratum_editor::{
    byte_to_utf16, dto_to_blocks, utf16_to_byte, BlockDto, EditorDocument, EditorState, RenderMode,
    VirtualLayout, EDITOR_I18N_KEYS,
};
use stratum_i18n::{t, Locale};
use stratum_ui::panels::editor::rows::{
    marker_prefix_len, source_offset_for_visible, visible_offset_for_source, visible_text,
};
use stratum_ui::panels::editor::{
    build_rows_for_state, SaveState, SaveStatus, EDITOR_VIEW_I18N_KEYS,
};
use stratum_ui::provider::{fixture_blocks, FixtureProvider, ShellData};

/// Build an `EditorDocument` from raw fixture DTOs.
fn document_from(path: &str, blocks: Vec<BlockDto>) -> EditorDocument {
    EditorDocument::from_blocks(path, dto_to_blocks(blocks)).expect("fixture blocks are valid")
}

/// A minimal paragraph DTO.
fn paragraph(id: &str, content: &str) -> BlockDto {
    BlockDto {
        id: id.to_string(),
        content: content.to_string(),
        parent_id: None,
        left_id: None,
        properties: Vec::new(),
        marker: None,
        priority: None,
        collapsed: false,
        heading_level: None,
    }
}

#[test]
fn ten_thousand_block_fixture_layout_is_bounded() {
    let provider = FixtureProvider::with_welcome_blocks(10_000);
    let blocks = provider
        .load_blocks("pages/Welcome.md")
        .expect("welcome page");
    assert_eq!(blocks.len(), 10_000);

    let document = document_from("pages/Welcome.md", blocks);
    let state = EditorState::new(document);
    let rows = build_rows_for_state(&state, true);
    assert_eq!(rows.len(), 10_000, "row count must equal block count");

    let layout = VirtualLayout::from_rows(&rows);
    let heights = layout.heights();
    assert_eq!(heights.len(), rows.len());
    assert!(heights
        .iter()
        .all(|height| height.is_finite() && *height >= MIN_ROW_HEIGHT));

    let offsets = layout.offsets();
    assert_eq!(offsets.len(), rows.len());
    assert_eq!(offsets[0], 0.0);
    for pair in offsets.windows(2) {
        assert!(pair[1] >= pair[0], "offsets must be monotonic");
    }
    assert!(layout.total_height() > 0.0);

    // The visible window must stay inside the documented bound for a 720 px
    // viewport even when scrolled to the middle and to the end.
    let max_rows = (720.0f32 / MIN_ROW_HEIGHT).ceil() as usize + 2 * 2 + 2;
    for scroll in [0.0, layout.total_height() / 2.0, layout.total_height()] {
        let window = layout.visible_window(scroll, 720.0, 2);
        assert!(window.start <= window.end);
        assert!(window.end <= rows.len());
        assert!(
            window.len() <= max_rows,
            "window {} exceeds bound {}",
            window.len(),
            max_rows
        );
    }

    // Row lookups agree with the offsets.
    for index in [0usize, 1, 4_999, 9_999] {
        let offset = layout.scroll_top_for_block(index).expect("offset");
        assert_eq!(layout.block_at_offset(offset + 1.0), Some(index));
        assert!(layout.anchor_offset(index).is_some());
    }
}

#[test]
fn focused_line_hides_delimiters_only_for_the_focused_block() {
    let blocks = vec![
        paragraph(
            "11111111-1111-1111-1111-111111111111",
            "**bold** and `code`",
        ),
        paragraph("22222222-2222-2222-2222-222222222222", "**other**"),
    ];
    let document = document_from("pages/Test.md", blocks);
    let ids = document.ordered_ids();
    let focus = ids[0];

    let stage_a = stratum_editor::render_page(&document, Some(focus), RenderMode::StageA, now());
    let focused =
        stratum_editor::render_page(&document, Some(focus), RenderMode::FocusedLine, now());

    // Stage A keeps every delimiter visible (muted by the view).
    for row in &stage_a {
        for span in &row.spans {
            if span.syntax {
                assert!(!span.hidden, "Stage A must not hide delimiters");
            }
        }
    }
    assert!(visible_text(&stage_a[0]).contains("**bold**"));
    assert!(visible_text(&stage_a[0]).contains('`'));

    // The focused line hides its delimiters; the next line stays Stage A.
    for span in &focused[0].spans {
        if span.syntax {
            assert!(span.hidden, "focused line must hide delimiters");
        }
    }
    assert_eq!(visible_text(&focused[0]), "bold and code");
    for span in &focused[1].spans {
        if span.syntax {
            assert!(!span.hidden, "non-focused lines stay Stage A");
        }
    }
    assert_eq!(visible_text(&focused[1]), "**other**");
}

#[test]
fn marker_keyword_is_stripped_and_offsets_map_to_source() {
    let blocks = fixture_blocks("pages/Welcome.md", 3);
    let document = document_from("pages/Welcome.md", blocks);
    let state = EditorState::new(document);
    let rows = build_rows_for_state(&state, true);
    let row = rows.get(2).expect("marker row");
    assert!(row.marker.is_some(), "fixture row 2 carries a marker");

    let text = visible_text(row);
    assert!(!text.contains("TODO"), "marker keyword lives in the badge");
    assert_eq!(marker_prefix_len("TODO Review", Some("TODO")), 5);
    assert_eq!(marker_prefix_len("Review", Some("TODO")), 0);
    assert_eq!(marker_prefix_len("TODOS list", Some("TODO")), 0);

    // Decorated spans preserve source lengths, so the visible/source mapping is
    // lossless within the stripped content.
    let total: usize = row.spans.iter().map(|span| span.text.len()).sum();
    assert_eq!(
        visible_offset_for_source(row, total),
        visible_text(row).len()
    );
    assert_eq!(
        source_offset_for_visible(row, visible_text(row).len()),
        total
    );
}

#[test]
fn slash_and_marker_catalogues_use_translated_keys() {
    let mut keys: Vec<&str> = Vec::new();
    for (has_selection, is_journal) in [(false, false), (true, true)] {
        for item in stratum_editor::menus::slash_items(has_selection, is_journal) {
            keys.push(item.title_key);
            if let Some(subtitle) = item.subtitle_key {
                keys.push(subtitle);
            }
        }
    }
    for item in stratum_editor::menus::marker_menu_items() {
        // Marker titles are the shipped literal keywords (TODO, DOING, …); only
        // priority titles are translation keys.
        if item.marker.is_none() {
            keys.push(item.title_key);
        }
        keys.push(item.subtitle_key);
        keys.push(item.group_key);
    }
    keys.extend_from_slice(EDITOR_I18N_KEYS);
    keys.extend_from_slice(EDITOR_VIEW_I18N_KEYS);

    for key in keys {
        assert!(
            EDITOR_I18N_KEYS.contains(&key) || EDITOR_VIEW_I18N_KEYS.contains(&key),
            "{key} is missing from the release gate lists"
        );
        for locale in Locale::ALL {
            assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn save_status_transitions_keep_the_document_dirty_on_failure() {
    let mut state = SaveState::new();
    assert_eq!(state.status(), &SaveStatus::Clean);
    state.mark_dirty();
    assert_eq!(state.status(), &SaveStatus::Dirty);
    assert!(state.needs_save());

    state.begin_save();
    assert_eq!(state.status(), &SaveStatus::Saving);
    state.fail("disk full");
    assert_eq!(state.status(), &SaveStatus::Failed("disk full".into()));
    assert!(state.needs_save());

    state.begin_save();
    let now = chrono::Utc::now();
    state.finish_save(now);
    assert_eq!(state.status(), &SaveStatus::Saved);
    assert_eq!(state.last_saved(), Some(now));
    assert!(!state.needs_save());
}

#[test]
fn utf16_caret_helpers_translate_cjk_and_emoji() {
    let text = "a😀你b";
    assert_eq!(utf16_to_byte(text, 0), Some(0));
    assert_eq!(utf16_to_byte(text, 1), Some(1));
    assert_eq!(
        utf16_to_byte(text, 2),
        None,
        "inside an emoji surrogate pair"
    );
    assert_eq!(utf16_to_byte(text, 3), Some(5));
    assert_eq!(utf16_to_byte(text, 5), Some(9));
    assert_eq!(utf16_to_byte(text, 6), None, "past the end");

    assert_eq!(byte_to_utf16(text, 0), Some(0));
    assert_eq!(byte_to_utf16(text, 1), Some(1));
    assert_eq!(byte_to_utf16(text, 5), Some(3));
    assert_eq!(byte_to_utf16(text, 9), Some(5));
    assert_eq!(byte_to_utf16(text, 4), None, "not a char boundary");

    for byte in [0usize, 1, 5, 8, 9] {
        let utf16 = byte_to_utf16(text, byte).expect("char boundary");
        assert_eq!(utf16_to_byte(text, utf16), Some(byte));
    }
}

/// A fixed timestamp for deterministic decorations.
fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(1_760_000_000, 0).expect("valid timestamp")
}
