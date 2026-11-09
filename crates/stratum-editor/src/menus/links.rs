//! Wiki-link and tag hit detection for ctrl-hover previews, ctrl-click
//! navigation and dead-link creation.
//!
//! These helpers only inspect inline spans; they never mutate the document.

use std::collections::HashSet;
use std::ops::Range;

use pkm_block::BlockId;

use crate::document::EditorDocument;
use crate::inline::{parse_inline, span_at, SpanKind};

/// A dead wiki-link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLink {
    pub block_id: BlockId,
    pub target: String,
    pub range: Range<usize>,
}

/// Find wiki-links whose targets are not in `known_targets`.
pub fn unresolved_wikilinks(
    doc: &EditorDocument,
    known_targets: &HashSet<String>,
) -> Vec<DeadLink> {
    let mut out = Vec::new();
    for block in doc.blocks() {
        for span in parse_inline(&block.content) {
            if let SpanKind::WikiLink { target, .. } = &span.kind {
                if !known_targets.contains(target) {
                    out.push(DeadLink {
                        block_id: block.id,
                        target: target.clone(),
                        range: span.start_byte..span.end_byte,
                    });
                }
            }
        }
    }
    out
}

/// The kind of link found under the caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkHitKind {
    WikiLink,
    Tag,
    ExternalUrl,
    BlockRef,
    Asset,
}

/// A ctrl-hover / ctrl-click link hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkHit {
    pub kind: LinkHitKind,
    pub target: String,
    pub range: Range<usize>,
}

/// Find the link span covering `byte_offset` in `block_id`.
pub fn link_at(doc: &EditorDocument, block_id: BlockId, byte_offset: usize) -> Option<LinkHit> {
    let block = doc.block(block_id)?;
    let spans = parse_inline(&block.content);
    let span = span_at(&spans, byte_offset)?;
    let (kind, target) = match &span.kind {
        SpanKind::WikiLink { target, .. } => (LinkHitKind::WikiLink, target.clone()),
        SpanKind::Tag => (LinkHitKind::Tag, span.text.clone()),
        SpanKind::Link { url } => (LinkHitKind::ExternalUrl, url.clone()),
        SpanKind::BlockRef { id } => (LinkHitKind::BlockRef, id.clone()),
        SpanKind::Asset { path } => (LinkHitKind::Asset, path.clone()),
        _ => return None,
    };
    Some(LinkHit {
        kind,
        target,
        range: span.start_byte..span.end_byte,
    })
}
