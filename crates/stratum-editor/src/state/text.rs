//! Byte-offset and delimiter helpers shared by the editing submodules.

use crate::inline::SpanKind;
use crate::state::Selection;

/// Clamp `offset` down to the nearest UTF-8 char boundary (and to the text
/// length). Structural ops such as `split_block` panic on a non-boundary, so
/// every caller funnels offsets through this function first.
pub(super) fn clamp_offset(text: &str, offset: usize) -> usize {
    let mut at = offset.min(text.len());
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// The char boundary immediately before `offset` (Backspace step).
pub(super) fn prev_char_boundary(text: &str, offset: usize) -> usize {
    let mut at = offset.min(text.len());
    if at == 0 {
        return 0;
    }
    at -= 1;
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// The char boundary immediately after `offset` (Delete step).
pub(super) fn next_char_boundary(text: &str, offset: usize) -> usize {
    let mut at = offset.min(text.len());
    if at >= text.len() {
        return text.len();
    }
    at += 1;
    while at < text.len() && !text.is_char_boundary(at) {
        at += 1;
    }
    at
}

/// Ordered `(start, end)` byte offsets of a same-block selection.
pub(super) fn ordered_offsets(sel: Selection) -> (usize, usize) {
    if sel.anchor.byte_offset <= sel.head.byte_offset {
        (sel.anchor.byte_offset, sel.head.byte_offset)
    } else {
        (sel.head.byte_offset, sel.anchor.byte_offset)
    }
}

/// The opening and closing delimiters used to wrap a span of `kind`.
pub(super) fn span_delimiters(kind: &SpanKind) -> (String, String) {
    match kind {
        SpanKind::Bold => ("**".into(), "**".into()),
        SpanKind::Italic => ("*".into(), "*".into()),
        SpanKind::Strike => ("~~".into(), "~~".into()),
        SpanKind::Code => ("`".into(), "`".into()),
        SpanKind::Highlight => ("==".into(), "==".into()),
        SpanKind::Math { display } => {
            if *display {
                ("$$".into(), "$$".into())
            } else {
                ("$".into(), "$".into())
            }
        }
        SpanKind::WikiLink { embed, .. } => {
            if *embed {
                ("![[".into(), "]]".into())
            } else {
                ("[[".into(), "]]".into())
            }
        }
        SpanKind::Tag => ("#".into(), String::new()),
        SpanKind::BlockRef { .. } => ("((".into(), "))".into()),
        SpanKind::Asset { .. } => ("![[".into(), "]]".into()),
        SpanKind::Link { url } => ("[".into(), format!("]({url})")),
        SpanKind::Text => (String::new(), String::new()),
    }
}
