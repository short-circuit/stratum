//! Wiki-link and tag hit detection for ctrl-hover previews, ctrl-click
//! navigation and dead-link creation.
//!
//! These helpers only inspect inline spans; they never mutate the document.

use std::collections::HashSet;
use std::ops::Range;

use pkm_block::BlockId;

use crate::document::EditorDocument;
use crate::inline::{parse_inline, span_at, SpanKind};

use super::WikiPage;

/// Outcome of resolving a wiki-link target against the current page list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WikiTargetResolution {
    /// The target resolves to this vault-relative page path.
    Resolved(String),
    /// No page matches the target, so the link is dead.
    Missing,
}

/// Resolve `target` against `pages` with the shipped/shell matching rules.
///
/// A page matches when its path equals the target, its path without `.md`
/// equals or ends with the target, or its slug or title equals the target. The
/// input is trimmed first. Returns the matched page path so callers can navigate
/// to it; `Missing` means Ctrl+click should offer page creation.
pub fn resolve_wiki_target(target: &str, pages: &[WikiPage]) -> WikiTargetResolution {
    let target = target.trim();
    if target.is_empty() {
        return WikiTargetResolution::Missing;
    }
    for page in pages {
        let path_without_ext = page.path.trim_end_matches(".md");
        if page.path == target
            || path_without_ext == target
            || page.slug == target
            || page.title == target
            || path_without_ext.ends_with(target)
        {
            return WikiTargetResolution::Resolved(page.path.clone());
        }
    }
    WikiTargetResolution::Missing
}

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

/// What a Ctrl+hover preview says about the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPreviewKind {
    /// The target resolves to an existing page; Ctrl+click navigates there.
    Resolved,
    /// No page matches the target yet; Ctrl+click offers to create it.
    Unresolved,
}

/// A Ctrl+hover preview model: the target text plus the hint to render.
///
/// The hint is a translation key so the view stays string-free; the shipped
/// `LinkPreviewPopup.tsx` renders `backlink_snippet_body.ctrl_click_to_navigate`
/// for resolved links, while the native dead-link affordance keeps
/// `editor.create_page` for targets that do not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkPreview {
    /// The link target as written in the block.
    pub target: String,
    /// Whether the target resolves to an existing page.
    pub kind: LinkPreviewKind,
    /// Translation key for the preview hint.
    pub hint_key: &'static str,
}

/// Translation key for the navigate hint on a resolved wiki link.
pub const PREVIEW_NAVIGATE_HINT_KEY: &str = "backlink_snippet_body.ctrl_click_to_navigate";

/// Translation key for the create-page hint on an unresolved wiki link.
pub const PREVIEW_CREATE_HINT_KEY: &str = "editor.create_page";

/// Build the Ctrl+hover preview for a link hit, if the editor previews it.
///
/// Only wiki links are previewed: `wikiLinks.ts` returns early unless the span
/// resolves to a page path, so tags, external URLs, block refs and asset embeds
/// have no preview popup. A wiki link that resolves gets the shipped navigate
/// hint; a dead one keeps the create-page hint because Ctrl+click offers
/// creation. Returning `None` means "show nothing" and must not be confused
/// with an unresolved target.
pub fn preview_for(hit: &LinkHit, pages: &[WikiPage]) -> Option<LinkPreview> {
    if hit.kind != LinkHitKind::WikiLink {
        return None;
    }
    let (kind, hint_key) = match resolve_wiki_target(&hit.target, pages) {
        WikiTargetResolution::Resolved(_) => (LinkPreviewKind::Resolved, PREVIEW_NAVIGATE_HINT_KEY),
        WikiTargetResolution::Missing => (LinkPreviewKind::Unresolved, PREVIEW_CREATE_HINT_KEY),
    };
    Some(LinkPreview {
        target: hit.target.clone(),
        kind,
        hint_key,
    })
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
