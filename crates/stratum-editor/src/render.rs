//! WYSIWYM decoration model. No UI types.
//!
//! Phase B renders [`DecoratedRow`] values directly: it never re-parses content
//! and never owns editing logic. The model exposes syntax delimiters separately
//! from content so the view can mute them in Stage A and hide them on the focused
//! line in FocusedLine mode.
//!
//! Marker keywords at the start of a block are removed from the rendered spans:
//! the marker is carried by the row badge instead.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use pkm_block::{Block, BlockId, Priority, TaskMarker};

use crate::assets::{first_asset_span, AssetSpan, AssetThumbnail};
use crate::document::EditorDocument;
use crate::inline::{parse_inline, SpanKind};
use crate::markers::{reminder_chips, MarkerBadge, ReminderChip};
use crate::math::{math_source, MathCache, MathImage, MathOptions};
use crate::mermaid::{mermaid_source, MermaidCache};

/// Rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// All syntax delimiters are visible and muted.
    StageA,
    /// Delimiters are hidden for the focused block only; other rows stay StageA.
    FocusedLine,
}

/// The structural kind of a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading(u8),
    Quote,
    Code {
        lang: String,
    },
    Mermaid,
    Math {
        display: bool,
    },
    ListItem,
    Divider,
    /// A GFM pipe table carried as plain Markdown block content.
    Table,
}

/// One decorated span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecoratedSpan {
    pub text: String,
    pub kind: SpanKind,
    /// Whether this span is a syntax delimiter (muted in Stage A).
    pub syntax: bool,
    /// Whether this span is hidden on the focused line.
    pub hidden: bool,
}

/// One decorated row.
///
/// `math`, `mermaid` and `thumbnail` carry lazily rendered media. `asset`
/// classifies the first `![[…]]` embed in the row so the view can reserve a
/// thumbnail or file row without re-parsing spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecoratedRow {
    pub block_id: BlockId,
    pub depth: usize,
    pub kind: BlockKind,
    pub marker: Option<TaskMarker>,
    pub priority: Option<Priority>,
    pub spans: Vec<DecoratedSpan>,
    pub badge: Option<MarkerBadge>,
    pub reminder: Option<ReminderChip>,
    pub collapsed: bool,
    /// Rendered math image, filled by [`render_page_with_assets`].
    pub math: Option<Arc<MathImage>>,
    /// Rendered Mermaid SVG, filled by [`render_page_with_assets`].
    pub mermaid: Option<Arc<String>>,
    /// First `![[…]]` asset embed in the row, classified by
    /// [`crate::assets::asset_kind`].
    pub asset: Option<AssetSpan>,
    /// Decoded image thumbnail. Left `None` by the renderer; the view fills it
    /// lazily through [`crate::assets::AssetThumbnailCache`].
    pub thumbnail: Option<Arc<AssetThumbnail>>,
}

/// Render the page into decorated rows in document order.
///
/// This is the pure decoration pass: math and Mermaid assets are left `None`,
/// as are decoded asset thumbnails. Use [`render_page_with_assets`] when the
/// view needs rendered images.
pub fn render_page(
    doc: &EditorDocument,
    focus: Option<BlockId>,
    mode: RenderMode,
    now: DateTime<Utc>,
) -> Vec<DecoratedRow> {
    render_rows(doc, focus, mode, now, None, None)
}

/// Render the page and fill `math`/`mermaid` assets through the supplied caches.
///
/// Asset rendering failures leave the field `None` so the view falls back to the
/// decorated text spans; the caches keep working after a failure.
pub fn render_page_with_assets(
    doc: &EditorDocument,
    focus: Option<BlockId>,
    mode: RenderMode,
    now: DateTime<Utc>,
    math_cache: &mut MathCache,
    mermaid_cache: &mut MermaidCache,
) -> Vec<DecoratedRow> {
    render_rows(doc, focus, mode, now, Some(math_cache), Some(mermaid_cache))
}

fn render_rows(
    doc: &EditorDocument,
    focus: Option<BlockId>,
    mode: RenderMode,
    now: DateTime<Utc>,
    mut math_cache: Option<&mut MathCache>,
    mut mermaid_cache: Option<&mut MermaidCache>,
) -> Vec<DecoratedRow> {
    let reminders: Vec<ReminderChip> = reminder_chips(doc, now);
    let mut rows = Vec::new();
    for block in doc.blocks() {
        let depth = doc.tree().depth(block.id);
        let kind = classify(block);
        let badge = if block.marker.is_some() || block.priority.is_some() {
            Some(MarkerBadge {
                block_id: block.id,
                marker: block.marker,
                priority: block.priority,
            })
        } else {
            None
        };
        let reminder = reminders
            .iter()
            .find(|chip| chip.block_id == block.id)
            .cloned();
        let math = math_cache.as_deref_mut().and_then(|cache| {
            math_source(&block.content)
                .and_then(|source| cache.render(&source, &MathOptions::default()).ok())
        });
        let mermaid = mermaid_cache.as_deref_mut().and_then(|cache| {
            mermaid_source(&block.content).and_then(|source| cache.render(source, "modern").ok())
        });
        let spans = build_spans(block, &kind, focus, mode);
        let asset = first_asset_span(&spans);
        rows.push(DecoratedRow {
            block_id: block.id,
            depth,
            kind: kind.clone(),
            marker: block.marker,
            priority: block.priority,
            spans,
            badge,
            reminder,
            collapsed: block.meta.collapsed,
            math,
            mermaid,
            asset,
            thumbnail: None,
        });
    }
    rows
}

fn build_spans(
    block: &Block,
    kind: &BlockKind,
    focus: Option<BlockId>,
    mode: RenderMode,
) -> Vec<DecoratedSpan> {
    // Code and mermaid bodies are opaque: keep them as a single text span so the
    // view can render them verbatim.
    if matches!(kind, BlockKind::Code { .. } | BlockKind::Mermaid) {
        return vec![DecoratedSpan {
            text: block.content.clone(),
            kind: SpanKind::Text,
            syntax: false,
            hidden: false,
        }];
    }
    let content = strip_marker_keyword(&block.content, block.marker);
    // Tables are rendered from their raw structural text: inline emphasis inside
    // cells is not decorated yet, so `parse_inline` never runs on table content.
    if matches!(kind, BlockKind::Table) {
        return table_spans(&content);
    }
    let hide_delimiters = mode == RenderMode::FocusedLine && focus == Some(block.id);
    let mut spans = Vec::new();
    for span in parse_inline(&content) {
        if span.kind == SpanKind::Text {
            spans.push(DecoratedSpan {
                text: span.text,
                kind: SpanKind::Text,
                syntax: false,
                hidden: false,
            });
            continue;
        }
        let (open, close) = delimiters(&span.kind);
        spans.push(DecoratedSpan {
            text: open,
            kind: span.kind.clone(),
            syntax: true,
            hidden: hide_delimiters,
        });
        spans.push(DecoratedSpan {
            text: span.text.clone(),
            kind: span.kind.clone(),
            syntax: false,
            hidden: false,
        });
        if !close.is_empty() {
            spans.push(DecoratedSpan {
                text: close,
                kind: span.kind,
                syntax: true,
                hidden: hide_delimiters,
            });
        }
    }
    spans
}

/// Build decorated spans for a GFM pipe table.
///
/// Pipes and the delimiter row are emitted as muted syntax spans; cell text
/// stays plain. The concatenation of every span text equals `content`
/// byte-for-byte, which the caret mapping relies on.
///
/// Unlike inline delimiters, table spans are never hidden on the focused line:
/// pipes are structural, and hiding them would visually merge adjacent cells.
fn table_spans(content: &str) -> Vec<DecoratedSpan> {
    let mut spans = Vec::new();
    for (line_index, line) in content.split_inclusive('\n').enumerate() {
        let delimiter_row = line_index == 1;
        let mut cell = String::new();
        for ch in line.chars() {
            if ch == '|' {
                if !cell.is_empty() {
                    spans.push(DecoratedSpan {
                        text: std::mem::take(&mut cell),
                        kind: SpanKind::Text,
                        syntax: delimiter_row,
                        hidden: false,
                    });
                }
                spans.push(DecoratedSpan {
                    text: "|".into(),
                    kind: SpanKind::Text,
                    syntax: true,
                    hidden: false,
                });
            } else {
                cell.push(ch);
            }
        }
        if !cell.is_empty() {
            spans.push(DecoratedSpan {
                text: cell,
                kind: SpanKind::Text,
                syntax: delimiter_row,
                hidden: false,
            });
        }
    }
    spans
}

fn delimiters(kind: &SpanKind) -> (String, String) {
    match kind {
        SpanKind::Bold => ("**".into(), "**".into()),
        SpanKind::Italic => ("*".into(), "*".into()),
        SpanKind::Strike => ("~~".into(), "~~".into()),
        SpanKind::Code => ("`".into(), "`".into()),
        SpanKind::Highlight => ("==".into(), "==".into()),
        SpanKind::Link { url } => ("[".into(), format!("]({url})")),
        SpanKind::WikiLink { embed, .. } => {
            if *embed {
                ("![[".into(), "]]".into())
            } else {
                ("[[".into(), "]]".into())
            }
        }
        SpanKind::Tag => ("#".into(), String::new()),
        SpanKind::Math { display } => {
            if *display {
                ("$$".into(), "$$".into())
            } else {
                ("$".into(), "$".into())
            }
        }
        SpanKind::BlockRef { .. } => ("((".into(), "))".into()),
        SpanKind::Asset { .. } => ("![[".into(), "]]".into()),
        SpanKind::Text => (String::new(), String::new()),
    }
}

fn strip_marker_keyword(content: &str, marker: Option<TaskMarker>) -> String {
    let Some(marker) = marker else {
        return content.to_string();
    };
    let keyword = marker.as_str();
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix(keyword) {
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            return rest.trim_start().to_string();
        }
    }
    content.to_string()
}

fn classify(block: &Block) -> BlockKind {
    if let Some(level) = block.meta.heading_level {
        return BlockKind::Heading(level);
    }
    let content = block.content.trim();
    if content.starts_with("```") {
        let first = content.lines().next().unwrap_or("");
        let lang = first.trim_start_matches("```").trim();
        if lang.eq_ignore_ascii_case("mermaid") {
            return BlockKind::Mermaid;
        }
        return BlockKind::Code {
            lang: lang.to_string(),
        };
    }
    if content.starts_with('>') {
        return BlockKind::Quote;
    }
    if content == "---" || is_thematic_break(content) {
        return BlockKind::Divider;
    }
    if content.starts_with("$$") && content.ends_with("$$") && content.len() >= 4 {
        return BlockKind::Math { display: true };
    }
    if content.starts_with('$') && content.ends_with('$') && content.len() >= 2 {
        return BlockKind::Math { display: false };
    }
    if pkm_markdown::is_table(content) {
        return BlockKind::Table;
    }
    if is_list_item(content) {
        return BlockKind::ListItem;
    }
    BlockKind::Paragraph
}

fn is_list_item(content: &str) -> bool {
    content.starts_with("- ")
        || content.starts_with("* ")
        || content.starts_with("+ ")
        || is_ordered_list_item(content)
}

fn is_ordered_list_item(content: &str) -> bool {
    let digits: String = content.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return false;
    }
    content[digits.len()..].starts_with(". ")
}

fn is_thematic_break(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, '-' | '*' | '_') {
        return false;
    }
    chars.all(|c| c == first || c == ' ')
}

#[cfg(test)]
mod tests;
