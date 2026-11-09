//! Headless block-editor engine for Stratum.
//!
//! This crate is deliberately UI-free: it contains no gpui, tauri or web
//! dependencies. Every editing rule, inline-span parse, undo record, IME
//! transition, menu catalog, marker/reminder model, WYSIWYM decoration and
//! virtualization calculation lives here so the phase-B gpui view can be a thin
//! renderer over [`render::DecoratedRow`] values.
//!
//! The modules map onto the shipped React editor (`src/components/OutlinerEditor`,
//! `src/components/AISlashMenu.tsx`, `src/lib/wikiLinks.ts`,
//! `src/lib/reminders/reminderUtils.ts`), which remains the behavioural
//! reference:
//!
//! - [`document`] — [`document::EditorDocument`], the byte-stability contract
//!   and canonicalization.
//! - [`dto`] — serde DTOs that are field-compatible with
//!   `stratum_app::block::BlockDto` and `src/lib/types.ts`.
//! - [`inline`] — inline span parsing and byte-exact re-serialization.
//! - [`markers`] — marker detection, badge rows and reminder chips.
//! - [`state`] — [`state::EditorState`]: caret/selection, editing operations,
//!   undo/redo, clipboard, paste normalization and IME composition.
//! - [`menus`] — slash-menu catalog, marker menu, wiki autocomplete and
//!   dead-link/link-hit detection.
//! - [`render`] — the WYSIWYM decoration model (no UI types).
//! - [`virtualize`] — deterministic row-height estimates and viewport windows.
//!
//! # Translation keys, never literals
//!
//! The crate never carries English UI strings. Models expose translation
//! *keys*; [`EDITOR_I18N_KEYS`] is the release gate list, mirroring
//! `stratum_ui::SHELL_I18N_KEYS`. Every key must exist in the shipped locale
//! catalogs (`crates/stratum-i18n/locales/*.json`).
//!
//! # Errors
//!
//! Fallible operations return [`pkm_core::PkmResult`]. No crate-local error
//! type is defined; missing variants belong in `pkm_core::PkmError`.

pub mod document;
pub mod dto;
pub mod inline;
pub mod markers;
pub mod math;
pub mod menus;
pub mod mermaid;
pub mod render;
pub mod state;
pub mod virtualize;

pub use document::{canonicalize_body, EditorDocument};
pub use dto::{block_to_dto, blocks_to_dto, dto_to_blocks, BlockDto, BlockListDto, PageDoc};
pub use inline::{parse_inline, render_inline, Span, SpanKind};
pub use markers::{
    badge_row, detect_and_apply_markers, reminder_chips, set_reminder, MarkerBadge, MarkerOption,
    PriorityOption, ReminderChip, MARKER_KEYWORDS, MARKER_OPTIONS, PRIORITY_OPTIONS,
};
pub use math::{math_source, MathCache, MathImage, MathOptions, MathSource};
pub use mermaid::{mermaid_source, MermaidCache};
pub use render::{
    render_page, render_page_with_assets, BlockKind, DecoratedRow, DecoratedSpan, RenderMode,
};
pub use state::{
    byte_to_utf16, normalize_paste, paste_markdown, utf16_to_byte, Caret, ClipboardPayload,
    Composition, EditOp, EditorState, History, HistoryEntry, Selection,
};
pub use virtualize::VirtualLayout;

/// Every translation key the editor models surface.
///
/// This is a release gate list, not a display table: the contract test walks it
/// against `crates/stratum-i18n/locales/en.json` and every [`menus::SlashItem`]
/// key must be a member. Adding a visible string without a catalog entry fails
/// the build instead of leaking a raw key into the UI.
pub const EDITOR_I18N_KEYS: &[&str] = &[
    // Block-insert / slash titles and subtitles.
    "aislash_menu.clean_up_formatting",
    "aislash_menu.clean_up_formatting_and_markdown",
    "aislash_menu.condense_into_key_points",
    "aislash_menu.create_a_concise_page_summary",
    "aislash_menu.create_a_diagram_from_a_description",
    "aislash_menu.format_notes",
    "aislash_menu.format_selection",
    "aislash_menu.format_structure",
    "aislash_menu.generate_mermaid_diagram",
    "aislash_menu.improve_clarity_and_flow",
    "aislash_menu.insert_a_latex_math_equation",
    "aislash_menu.interlink_notes",
    "aislash_menu.organize_and_clean_up_the_page",
    "aislash_menu.organize_daily_notes_into_sections",
    "aislash_menu.research_with_web",
    "aislash_menu.rewrite",
    "aislash_menu.search_the_web_and_write_research_notes",
    "aislash_menu.search_vault_and_add_wiki_links_to_related_notes",
    "aislash_menu.structure_journal",
    "aislash_menu.summarize",
    "aislash_menu.summarize_page",
    "editor.code_block",
    "editor.heading_1",
    "editor.heading_2",
    "editor.heading_3",
    "editor.math_equation_label",
    "editor.mermaid_diagram",
    "editor.paragraph",
    // Marker / priority menu labels.
    "editor.high_priority",
    "editor.low_priority",
    "editor.markers",
    "editor.medium_priority",
    "editor.priority",
    "editor.priority_level",
    "editor.task_marker",
    // Reminder humanization.
    "reminders.humanize.due_date",
    "reminders.humanize.due_in_hours",
    "reminders.humanize.due_in_min",
    "reminders.humanize.now",
    "reminders.humanize.overdue_hours",
    "reminders.humanize.overdue_min",
    "reminders.humanize.overdue_now",
    // Save status, clipboard and read-aloud actions.
    "editor.copy",
    "editor.cut",
    "editor.delete",
    "editor.read_aloud",
    "editor.saving",
    // Page empty state.
    "page.prose_empty",
];
