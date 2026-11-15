//! AI slash-action requests.
//!
//! The shipped React editor's `AISlashMenu` maps every AI menu item onto one
//! service call and one insertion strategy. That mapping is UI-free, so it lives
//! here and the gpui view only executes it through the `ShellData` seam.

use super::SlashAction;

/// The `ai_transform_block` action ids used by `stratum-app` (`AiAction` serde).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTransformAction {
    Rewrite,
    Format,
    Structure,
    Summarize,
    Connect,
}

impl AiTransformAction {
    /// The snake-case id accepted by `stratum_app::ai::AiAction`.
    pub fn service_name(self) -> &'static str {
        match self {
            Self::Rewrite => "rewrite",
            Self::Format => "format",
            Self::Structure => "structure",
            Self::Summarize => "summarize",
            Self::Connect => "connect",
        }
    }
}

/// The service call behind an AI menu item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCallKind {
    Transform(AiTransformAction),
    Research,
    Interlink,
    Mermaid,
}

/// Where the result lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCallScope {
    /// Replace the whole page body with the returned markdown.
    ReplacePage,
    /// Insert the returned markdown at the caret (replacing any selection).
    InsertAtCaret,
    /// Insert one new sibling block after the caret block (Mermaid diagram).
    InsertBlockAfterCaret,
}

/// One executable AI request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCall {
    pub kind: AiCallKind,
    /// Where the result lands. Carried explicitly because a transform action
    /// (format/summarize) can be either selection- or page-scoped depending on
    /// the slash item that produced it.
    pub scope: AiCallScope,
    pub text: String,
    pub page_path: Option<String>,
}

/// Resolve a non-empty selection.
fn non_empty(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// Resolve the research prompt: page text first, then selection, then page path.
///
/// Mirrors `const query = allText || capturedRef.current || pagePath;` in
/// `src/components/AISlashMenu.tsx`.
fn research_prompt(page_text: &str, selection: Option<&str>, page_path: &str) -> Option<String> {
    let page_text = page_text.trim();
    if !page_text.is_empty() {
        return Some(page_text.to_string());
    }
    if let Some(selection) = non_empty(selection) {
        return Some(selection);
    }
    let page_path = page_path.trim();
    if page_path.is_empty() {
        None
    } else {
        Some(page_path.to_string())
    }
}

/// Map one slash action onto the service request it executes.
///
/// Mirrors `src/components/AISlashMenu.tsx`: selection actions require a
/// non-empty selection; page transforms send the page's plain text
/// (`extractText` parity); interlink sends property-free markdown
/// (`blocksToMarkdownLossy` parity); research prefers the page text, then a
/// captured selection, then the page path; and Mermaid prefers a selection,
/// then the page text. Returns `None` for block actions and for an empty
/// resolved prompt, so the service is never called with an empty string.
pub fn ai_call_for(
    action: SlashAction,
    selection: Option<&str>,
    page_text: &str,
    page_markdown_lossy: &str,
    page_path: &str,
) -> Option<AiCall> {
    let page_text = page_text.trim();
    let page_markdown = page_markdown_lossy.trim();
    let call = match action {
        SlashAction::AiRewrite => AiCall {
            kind: AiCallKind::Transform(AiTransformAction::Rewrite),
            scope: AiCallScope::InsertAtCaret,
            text: non_empty(selection)?,
            page_path: Some(page_path.to_string()),
        },
        SlashAction::AiFormatSelection => AiCall {
            kind: AiCallKind::Transform(AiTransformAction::Format),
            scope: AiCallScope::InsertAtCaret,
            text: non_empty(selection)?,
            page_path: Some(page_path.to_string()),
        },
        SlashAction::AiSummarizeSelection => AiCall {
            kind: AiCallKind::Transform(AiTransformAction::Summarize),
            scope: AiCallScope::InsertAtCaret,
            text: non_empty(selection)?,
            page_path: Some(page_path.to_string()),
        },
        SlashAction::AiStructureJournal | SlashAction::AiStructurePage => AiCall {
            kind: AiCallKind::Transform(AiTransformAction::Structure),
            scope: AiCallScope::ReplacePage,
            text: page_text.to_string(),
            page_path: Some(page_path.to_string()),
        },
        SlashAction::AiFormatJournalNotes => AiCall {
            kind: AiCallKind::Transform(AiTransformAction::Format),
            scope: AiCallScope::ReplacePage,
            text: page_text.to_string(),
            page_path: Some(page_path.to_string()),
        },
        SlashAction::AiSummarizePage => AiCall {
            kind: AiCallKind::Transform(AiTransformAction::Summarize),
            scope: AiCallScope::ReplacePage,
            text: page_text.to_string(),
            page_path: Some(page_path.to_string()),
        },
        SlashAction::AiResearch => AiCall {
            kind: AiCallKind::Research,
            scope: AiCallScope::ReplacePage,
            text: research_prompt(page_text, selection, page_path)?,
            page_path: None,
        },
        SlashAction::AiInterlink => AiCall {
            kind: AiCallKind::Interlink,
            scope: AiCallScope::ReplacePage,
            text: page_markdown.to_string(),
            page_path: Some(page_path.to_string()),
        },
        SlashAction::AiGenerateMermaid => AiCall {
            kind: AiCallKind::Mermaid,
            scope: AiCallScope::InsertBlockAfterCaret,
            text: non_empty(selection)
                .or_else(|| (!page_text.is_empty()).then(|| page_text.to_string()))?,
            page_path: None,
        },
        _ => return None,
    };
    if call.text.trim().is_empty() {
        return None;
    }
    Some(call)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_action_ids_are_snake_case() {
        assert_eq!(AiTransformAction::Rewrite.service_name(), "rewrite");
        assert_eq!(AiTransformAction::Format.service_name(), "format");
        assert_eq!(AiTransformAction::Structure.service_name(), "structure");
        assert_eq!(AiTransformAction::Summarize.service_name(), "summarize");
        assert_eq!(AiTransformAction::Connect.service_name(), "connect");
    }

    #[test]
    fn page_scoped_format_and_summarize_replace_the_page() {
        let page_text = "one\ntwo";
        let page_markdown = "- one\n- two";
        let path = "pages/x.md";
        for action in [
            SlashAction::AiFormatJournalNotes,
            SlashAction::AiSummarizePage,
            SlashAction::AiStructureJournal,
            SlashAction::AiStructurePage,
            SlashAction::AiResearch,
            SlashAction::AiInterlink,
        ] {
            let call =
                ai_call_for(action, None, page_text, page_markdown, path).expect("page call");
            assert_eq!(call.scope, AiCallScope::ReplacePage, "{action:?}");
        }
        for action in [
            SlashAction::AiRewrite,
            SlashAction::AiFormatSelection,
            SlashAction::AiSummarizeSelection,
        ] {
            let call = ai_call_for(action, Some("text"), page_text, page_markdown, path)
                .expect("selection");
            assert_eq!(call.scope, AiCallScope::InsertAtCaret, "{action:?}");
        }
        let mermaid = ai_call_for(
            SlashAction::AiGenerateMermaid,
            None,
            page_text,
            page_markdown,
            path,
        )
        .expect("mermaid");
        assert_eq!(mermaid.scope, AiCallScope::InsertBlockAfterCaret);
    }

    #[test]
    fn research_prefers_page_text_over_selection_and_path() {
        let call = ai_call_for(
            SlashAction::AiResearch,
            Some("selected"),
            "page body",
            "- page body",
            "pages/x.md",
        )
        .expect("research");
        assert_eq!(call.text, "page body");

        let call = ai_call_for(
            SlashAction::AiResearch,
            Some("selected"),
            "  ",
            "- body",
            "pages/x.md",
        )
        .expect("selection fallback");
        assert_eq!(call.text, "selected");

        let call = ai_call_for(SlashAction::AiResearch, None, "", "", "pages/x.md")
            .expect("path fallback");
        assert_eq!(call.text, "pages/x.md");
    }
}
