//! Menu catalogs and link helpers.
//!
//! Ports the shipped catalogs and pure helpers:
//! - `src/components/AISlashMenu.tsx` and
//!   `src/components/OutlinerEditor/editorActionMenuItems.tsx` for slash and
//!   insert actions.
//! - `src/components/OutlinerEditor/MarkerSuggestMenu.tsx` for marker/priority
//!   menu labels.
//! - `src/components/OutlinerEditor/wikiLinkTargets.ts` for duplicate-slug
//!   disambiguation.
//! - `src/lib/wikiLinks.ts` for wiki-query, dead-link and ctrl-hover link hits.
//!
//! Every visible label is a translation key (never English text).

use std::collections::HashMap;

use std::ops::Range;

use pkm_block::{Priority, TaskMarker};

mod ai;
pub use ai::{ai_call_for, AiCall, AiCallKind, AiCallScope, AiTransformAction};

mod links;
pub use links::{
    link_at, resolve_wiki_target, unresolved_wikilinks, DeadLink, LinkHit, LinkHitKind,
    WikiTargetResolution,
};

/// Largest number of slash-menu rows the native popup can draw at once.
///
/// The popup renders exactly `slash_row_count(items)` rows and the overlay
/// selection is clamped to the same count, so a selectable index is always a
/// drawn row. Catalogue contexts are asserted against this bound in tests.
pub const SLASH_MAX_ROWS: usize = 16;

/// Number of slash rows the popup draws for a catalogue of `items` entries.
///
/// Single source of truth shared by the overlay selection length and the
/// renderer; never call `.take(8)` or any other ad-hoc cap in the view.
pub fn slash_row_count(items: usize) -> usize {
    items.min(SLASH_MAX_ROWS)
}

/// Where a slash action operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashScope {
    Selection,
    Page,
}

/// Slash-menu grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashGroup {
    Blocks,
    Ai,
}

/// One slash-menu entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashItem {
    pub id: &'static str,
    pub title_key: &'static str,
    pub subtitle_key: Option<&'static str>,
    pub aliases: Vec<&'static str>,
    pub group: SlashGroup,
    pub scope: SlashScope,
    pub requires_selection: bool,
    pub journal_only: bool,
}

/// Every shipped slash action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlashAction {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    CodeBlock,
    Mermaid,
    MathEquation,
    Quote,
    AiRewrite,
    AiFormatSelection,
    AiSummarizeSelection,
    AiStructureJournal,
    AiFormatJournalNotes,
    AiStructurePage,
    AiSummarizePage,
    AiResearch,
    AiInterlink,
    AiGenerateMermaid,
}

impl SlashAction {
    /// Every action in shipped display order.
    pub const ALL: [SlashAction; 18] = [
        SlashAction::Paragraph,
        SlashAction::Heading1,
        SlashAction::Heading2,
        SlashAction::Heading3,
        SlashAction::CodeBlock,
        SlashAction::Mermaid,
        SlashAction::MathEquation,
        SlashAction::Quote,
        SlashAction::AiRewrite,
        SlashAction::AiFormatSelection,
        SlashAction::AiSummarizeSelection,
        SlashAction::AiStructureJournal,
        SlashAction::AiFormatJournalNotes,
        SlashAction::AiStructurePage,
        SlashAction::AiSummarizePage,
        SlashAction::AiResearch,
        SlashAction::AiInterlink,
        SlashAction::AiGenerateMermaid,
    ];

    /// Stable identifier.
    pub fn id(self) -> &'static str {
        match self {
            SlashAction::Paragraph => "paragraph",
            SlashAction::Heading1 => "heading_1",
            SlashAction::Heading2 => "heading_2",
            SlashAction::Heading3 => "heading_3",
            SlashAction::CodeBlock => "code_block",
            SlashAction::Mermaid => "mermaid_diagram",
            SlashAction::MathEquation => "math_equation",
            SlashAction::Quote => "quote",
            SlashAction::AiRewrite => "ai_rewrite",
            SlashAction::AiFormatSelection => "ai_format_selection",
            SlashAction::AiSummarizeSelection => "ai_summarize_selection",
            SlashAction::AiStructureJournal => "ai_structure_journal",
            SlashAction::AiFormatJournalNotes => "ai_format_journal_notes",
            SlashAction::AiStructurePage => "ai_structure_page",
            SlashAction::AiSummarizePage => "ai_summarize_page",
            SlashAction::AiResearch => "ai_research",
            SlashAction::AiInterlink => "ai_interlink",
            SlashAction::AiGenerateMermaid => "ai_generate_mermaid",
        }
    }

    /// Translation key for the title.
    pub fn title_key(self) -> &'static str {
        match self {
            SlashAction::Paragraph => "editor.paragraph",
            SlashAction::Heading1 => "editor.heading_1",
            SlashAction::Heading2 => "editor.heading_2",
            SlashAction::Heading3 => "editor.heading_3",
            SlashAction::CodeBlock => "editor.code_block",
            SlashAction::Mermaid => "editor.mermaid_diagram",
            SlashAction::MathEquation => "editor.math_equation_label",
            SlashAction::Quote => "editor.quote",
            SlashAction::AiRewrite => "aislash_menu.rewrite",
            SlashAction::AiFormatSelection => "aislash_menu.format_selection",
            SlashAction::AiSummarizeSelection => "aislash_menu.summarize",
            SlashAction::AiStructureJournal => "aislash_menu.structure_journal",
            SlashAction::AiFormatJournalNotes => "aislash_menu.format_notes",
            SlashAction::AiStructurePage => "aislash_menu.format_structure",
            SlashAction::AiSummarizePage => "aislash_menu.summarize_page",
            SlashAction::AiResearch => "aislash_menu.research_with_web",
            SlashAction::AiInterlink => "aislash_menu.interlink_notes",
            SlashAction::AiGenerateMermaid => "aislash_menu.generate_mermaid_diagram",
        }
    }

    /// Translation key for the subtitle, when the shipped menu has one.
    pub fn subtitle_key(self) -> Option<&'static str> {
        match self {
            SlashAction::Paragraph
            | SlashAction::Heading1
            | SlashAction::Heading2
            | SlashAction::Heading3
            | SlashAction::CodeBlock
            | SlashAction::Mermaid
            | SlashAction::Quote => None,
            SlashAction::MathEquation => Some("aislash_menu.insert_a_latex_math_equation"),
            SlashAction::AiRewrite => Some("aislash_menu.improve_clarity_and_flow"),
            SlashAction::AiFormatSelection => Some("aislash_menu.clean_up_formatting"),
            SlashAction::AiSummarizeSelection => Some("aislash_menu.condense_into_key_points"),
            SlashAction::AiStructureJournal => {
                Some("aislash_menu.organize_daily_notes_into_sections")
            }
            SlashAction::AiFormatJournalNotes => {
                Some("aislash_menu.clean_up_formatting_and_markdown")
            }
            SlashAction::AiStructurePage => Some("aislash_menu.organize_and_clean_up_the_page"),
            SlashAction::AiSummarizePage => Some("aislash_menu.create_a_concise_page_summary"),
            SlashAction::AiResearch => Some("aislash_menu.search_the_web_and_write_research_notes"),
            SlashAction::AiInterlink => {
                Some("aislash_menu.search_vault_and_add_wiki_links_to_related_notes")
            }
            SlashAction::AiGenerateMermaid => {
                Some("aislash_menu.create_a_diagram_from_a_description")
            }
        }
    }

    /// Shipped aliases.
    pub fn aliases(self) -> Vec<&'static str> {
        match self {
            SlashAction::MathEquation => vec!["math", "equation", "latex", "formula"],
            SlashAction::Quote => vec!["quote", "blockquote"],
            SlashAction::AiResearch => vec!["research", "search", "web"],
            SlashAction::AiInterlink => vec!["interlink", "wikilinks", "connect"],
            SlashAction::AiGenerateMermaid => vec!["mermaid", "diagram", "chart"],
            _ => Vec::new(),
        }
    }

    /// Menu group.
    pub fn group(self) -> SlashGroup {
        match self {
            SlashAction::Paragraph
            | SlashAction::Heading1
            | SlashAction::Heading2
            | SlashAction::Heading3
            | SlashAction::CodeBlock
            | SlashAction::Mermaid
            | SlashAction::MathEquation
            | SlashAction::Quote => SlashGroup::Blocks,
            _ => SlashGroup::Ai,
        }
    }

    /// Action scope.
    pub fn scope(self) -> SlashScope {
        match self {
            SlashAction::AiRewrite
            | SlashAction::AiFormatSelection
            | SlashAction::AiSummarizeSelection => SlashScope::Selection,
            _ => SlashScope::Page,
        }
    }

    /// Whether the action requires a text selection.
    pub fn requires_selection(self) -> bool {
        matches!(
            self,
            SlashAction::AiRewrite
                | SlashAction::AiFormatSelection
                | SlashAction::AiSummarizeSelection
        )
    }

    /// Whether the action is only offered on journal pages.
    pub fn journal_only(self) -> bool {
        matches!(
            self,
            SlashAction::AiStructureJournal | SlashAction::AiFormatJournalNotes
        )
    }

    /// Whether the action is offered on non-journal pages.
    pub fn page_only_non_journal(self) -> bool {
        matches!(
            self,
            SlashAction::AiStructurePage | SlashAction::AiSummarizePage
        )
    }
}

/// Build the slash items visible for the current context.
pub fn slash_items(has_selection: bool, is_journal: bool) -> Vec<SlashItem> {
    SlashAction::ALL
        .iter()
        .filter(|action| {
            if action.requires_selection() && !has_selection {
                return false;
            }
            if action.journal_only() && !is_journal {
                return false;
            }
            if action.page_only_non_journal() && is_journal {
                return false;
            }
            true
        })
        .map(|action| SlashItem {
            id: action.id(),
            title_key: action.title_key(),
            subtitle_key: action.subtitle_key(),
            aliases: action.aliases(),
            group: action.group(),
            scope: action.scope(),
            requires_selection: action.requires_selection(),
            journal_only: action.journal_only(),
        })
        .collect()
}

/// Filter slash items by a case-insensitive substring of the title-key tail or
/// an alias. Order is stable.
pub fn filter_slash_items(items: &[SlashItem], query: &str) -> Vec<SlashItem> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return items.to_vec();
    }
    items
        .iter()
        .filter(|item| {
            let tail = item
                .title_key
                .rsplit('.')
                .next()
                .unwrap_or(item.title_key)
                .to_lowercase();
            tail.contains(&needle)
                || item
                    .aliases
                    .iter()
                    .any(|alias| alias.to_lowercase().contains(&needle))
        })
        .cloned()
        .collect()
}

/// One marker/priority menu entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerMenuItem {
    pub id: &'static str,
    pub title_key: &'static str,
    pub title_vars: Vec<(String, String)>,
    pub subtitle_key: &'static str,
    pub group_key: &'static str,
    pub marker: Option<TaskMarker>,
    pub priority: Option<Priority>,
}

/// Build the marker/priority menu catalog (7 markers + 3 priorities).
pub fn marker_menu_items() -> Vec<MarkerMenuItem> {
    let mut items = Vec::new();
    for option in crate::markers::MARKER_OPTIONS {
        items.push(MarkerMenuItem {
            id: option.keyword,
            title_key: option.keyword,
            title_vars: Vec::new(),
            subtitle_key: option.subtitle_key,
            group_key: option.group_key,
            marker: Some(option.marker),
            priority: None,
        });
    }
    for option in crate::markers::PRIORITY_OPTIONS {
        items.push(MarkerMenuItem {
            id: match option.priority {
                Priority::A => "priority_a",
                Priority::B => "priority_b",
                Priority::C => "priority_c",
            },
            title_key: option.title_key,
            title_vars: option
                .title_vars
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            subtitle_key: option.subtitle_key,
            group_key: option.group_key,
            marker: None,
            priority: Some(option.priority),
        });
    }
    items
}

/// A detected wiki-link autocomplete query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiQuery {
    pub term: String,
    pub replace_range: Range<usize>,
}

/// Detect an unclosed `[[` before `byte_offset` and return its term and the
/// byte range to replace.
pub fn wiki_query(text: &str, byte_offset: usize) -> Option<WikiQuery> {
    let offset = byte_offset.min(text.len());
    let before = &text[..offset];
    let open = before.rfind("[[")?;
    if before[open + 2..].contains("]]") {
        return None;
    }
    let raw_term = &text[open + 2..offset];
    let term = raw_term.split('|').next().unwrap_or(raw_term).to_string();
    Some(WikiQuery {
        term,
        replace_range: (open + 2)..offset,
    })
}

/// A page that can be linked to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiPage {
    pub path: String,
    pub slug: String,
    pub title: String,
}

/// A resolved wiki autocomplete target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiTarget {
    pub path: String,
    pub slug: String,
    pub title: String,
    pub target: String,
}

/// Find pages matching `term` and compute the link target to insert.
pub fn wiki_targets(term: &str, pages: &[WikiPage]) -> Vec<WikiTarget> {
    let counts = slug_counts(pages);
    let needle = term.to_lowercase();
    pages
        .iter()
        .filter(|page| {
            page.slug.to_lowercase().contains(&needle)
                || page.title.to_lowercase().contains(&needle)
        })
        .map(|page| WikiTarget {
            path: page.path.clone(),
            slug: page.slug.clone(),
            title: page.title.clone(),
            target: compute_wiki_link_target(page, &counts),
        })
        .collect()
}

/// Choose the wiki-link target for a page.
///
/// A bare slug is used when unique. When two or more pages share the slug, the
/// vault-relative path minus `.md` is unambiguous and is used instead. An empty
/// slug/title yields an empty target.
pub fn compute_wiki_link_target(page: &WikiPage, slug_counts: &HashMap<String, usize>) -> String {
    let slug = if page.slug.trim().is_empty() {
        page.title.trim()
    } else {
        page.slug.trim()
    };
    if slug.is_empty() {
        return String::new();
    }
    let duplicate = slug_counts.get(slug).copied().unwrap_or(0) > 1;
    if duplicate {
        page.path
            .strip_suffix(".md")
            .unwrap_or(&page.path)
            .to_string()
    } else {
        slug.to_string()
    }
}

fn slug_counts(pages: &[WikiPage]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for page in pages {
        let slug = if page.slug.trim().is_empty() {
            page.title.trim()
        } else {
            page.slug.trim()
        };
        if !slug.is_empty() {
            *counts.entry(slug.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests;
