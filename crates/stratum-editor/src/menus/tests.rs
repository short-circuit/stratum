use std::collections::{HashMap, HashSet};

use pkm_block::Block;
use uuid::Uuid;

use super::*;
use crate::document::EditorDocument;

fn page(path: &str, slug: &str, title: &str) -> WikiPage {
    WikiPage {
        path: path.into(),
        slug: slug.into(),
        title: title.into(),
    }
}

#[test]
fn unique_slug_uses_the_bare_slug() {
    let mut counts = HashMap::new();
    counts.insert("unique".to_string(), 1);
    let target = compute_wiki_link_target(&page("pages/unique.md", "unique", "Unique"), &counts);
    assert_eq!(target, "unique");
}

#[test]
fn duplicated_slug_uses_the_path_without_extension() {
    let mut counts = HashMap::new();
    counts.insert("note".to_string(), 2);
    let target = compute_wiki_link_target(&page("pages/a/note.md", "note", "Note A"), &counts);
    assert_eq!(target, "pages/a/note");
    assert!(!target.ends_with(".md"));
}

#[test]
fn empty_slug_and_title_yield_empty_target() {
    let target = compute_wiki_link_target(&page("pages/x.md", "", ""), &HashMap::new());
    assert_eq!(target, "");
}

#[test]
fn wiki_targets_filter_by_slug_or_title() {
    let pages = vec![
        page("pages/alpha.md", "alpha", "Alpha"),
        page("pages/beta.md", "beta", "Beta"),
    ];
    let targets = wiki_targets("alp", &pages);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].target, "alpha");
}

#[test]
fn wiki_query_detects_an_unclosed_link() {
    let text = "hello [[Tar";
    let query = wiki_query(text, text.len()).unwrap();
    assert_eq!(query.term, "Tar");
    assert_eq!(query.replace_range, 8..11);
    assert!(wiki_query("[[done]]", 8).is_none());
    assert!(wiki_query("no link", 7).is_none());
}

#[test]
fn wiki_query_keeps_the_term_before_a_pipe() {
    let text = "[[Target|disp";
    let query = wiki_query(text, text.len()).unwrap();
    assert_eq!(query.term, "Target");
}

#[test]
fn dead_links_are_reported_against_known_targets() {
    let id = Uuid::new_v4();
    let doc = EditorDocument::from_blocks(
        "pages/x.md",
        vec![Block::new(id, "[[Known]] and [[Missing]]".into())],
    )
    .unwrap();
    let mut known = HashSet::new();
    known.insert("Known".to_string());
    let dead = unresolved_wikilinks(&doc, &known);
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].target, "Missing");
    assert_eq!(dead[0].block_id, id);
}

#[test]
fn link_at_finds_wiki_tags_and_external_links() {
    let wiki = Uuid::new_v4();
    let doc =
        EditorDocument::from_blocks("pages/x.md", vec![Block::new(wiki, "[[Target]]".into())])
            .unwrap();
    let hit = link_at(&doc, wiki, 2).unwrap();
    assert_eq!(hit.kind, LinkHitKind::WikiLink);
    assert_eq!(hit.target, "Target");

    let tag = Uuid::new_v4();
    let doc =
        EditorDocument::from_blocks("pages/x.md", vec![Block::new(tag, "#tag".into())]).unwrap();
    let hit = link_at(&doc, tag, 0).unwrap();
    assert_eq!(hit.kind, LinkHitKind::Tag);
    assert_eq!(hit.target, "tag");

    let url = Uuid::new_v4();
    let doc = EditorDocument::from_blocks(
        "pages/x.md",
        vec![Block::new(url, "[label](https://example.com)".into())],
    )
    .unwrap();
    let hit = link_at(&doc, url, 1).unwrap();
    assert_eq!(hit.kind, LinkHitKind::ExternalUrl);
    assert_eq!(hit.target, "https://example.com");
}

#[test]
fn every_slash_action_appears_in_its_context() {
    let plain = slash_items(false, false);
    let journal = slash_items(false, true);
    let selected = slash_items(true, false);
    let selected_journal = slash_items(true, true);

    let ids = |items: &[SlashItem]| -> HashSet<&'static str> {
        items.iter().map(|item| item.id).collect()
    };
    let plain_ids = ids(&plain);
    let journal_ids = ids(&journal);
    let selected_ids = ids(&selected);
    let selected_journal_ids = ids(&selected_journal);

    for action in SlashAction::ALL {
        let id = action.id();
        let in_plain = plain_ids.contains(id);
        let in_journal = journal_ids.contains(id);
        let in_selected = selected_ids.contains(id);
        let in_selected_journal = selected_journal_ids.contains(id);

        if action.requires_selection() {
            assert!(!in_plain, "{id} should require a selection");
            assert!(in_selected, "{id} missing with a selection");
        } else if action.journal_only() {
            assert!(!in_plain, "{id} should be journal-only");
            assert!(in_journal, "{id} missing on a journal");
        } else if action.page_only_non_journal() {
            assert!(in_plain, "{id} missing on a page");
            assert!(!in_journal, "{id} should be hidden on a journal");
        } else {
            assert!(in_plain, "{id} missing without a selection");
            assert!(in_journal, "{id} missing on a journal");
        }

        // Selection-requiring actions that are also journal-only must show up
        // only in the journal+selection context, never in a non-journal menu.
        if action.requires_selection() && action.journal_only() {
            assert!(
                in_selected_journal,
                "{id} missing on a journal with a selection"
            );
            assert!(!in_selected, "{id} should be hidden off-journal");
        }
    }
    assert_eq!(plain.len() + journal.len(), 26);
}

#[test]
fn every_slash_catalogue_fits_the_rendered_row_bound() {
    for items in [
        slash_items(false, false),
        slash_items(false, true),
        slash_items(true, false),
        slash_items(true, true),
    ] {
        assert_eq!(slash_row_count(items.len()), items.len());
        assert!(items.len() <= SLASH_MAX_ROWS);
    }
    assert_eq!(slash_row_count(99), SLASH_MAX_ROWS);
}

#[test]
fn quote_is_a_block_action_with_shipped_aliases() {
    let quote = SlashAction::Quote;
    assert_eq!(quote.id(), "quote");
    assert_eq!(quote.title_key(), "editor.quote");
    assert_eq!(quote.aliases(), vec!["quote", "blockquote"]);
    assert_eq!(quote.subtitle_key(), None);
    assert_eq!(quote.group(), SlashGroup::Blocks);
    assert!(!quote.requires_selection());
    assert!(!quote.journal_only());
    assert!(slash_items(false, false)
        .iter()
        .any(|item| item.id == "quote"));
}

#[test]
fn selection_actions_require_non_empty_text() {
    let page = "page body";
    let path = "pages/x.md";
    for action in [
        SlashAction::AiRewrite,
        SlashAction::AiFormatSelection,
        SlashAction::AiSummarizeSelection,
    ] {
        assert!(ai_call_for(action, None, page, path).is_none());
        assert!(ai_call_for(action, Some("   "), page, path).is_none());
        let call = ai_call_for(action, Some("selected"), page, path).expect("selection call");
        assert_eq!(call.text, "selected");
        assert_eq!(call.page_path, Some(path.to_string()));
        assert_eq!(call.scope, AiCallScope::InsertAtCaret);
    }
}

#[test]
fn page_actions_send_the_page_markdown() {
    let page = "- one\n- two";
    let path = "pages/x.md";
    for (action, transform) in [
        (SlashAction::AiStructurePage, AiTransformAction::Structure),
        (
            SlashAction::AiStructureJournal,
            AiTransformAction::Structure,
        ),
        (SlashAction::AiFormatJournalNotes, AiTransformAction::Format),
        (SlashAction::AiSummarizePage, AiTransformAction::Summarize),
    ] {
        let call = ai_call_for(action, None, page, path).expect("page call");
        assert_eq!(call.text, page);
        assert_eq!(call.kind, AiCallKind::Transform(transform));
        assert_eq!(call.scope, AiCallScope::ReplacePage);
        assert_eq!(call.page_path, Some(path.to_string()));
    }
}

#[test]
fn interlink_sends_the_page_markdown_and_path() {
    let call = ai_call_for(SlashAction::AiInterlink, None, "body", "pages/x.md").expect("call");
    assert_eq!(call.kind, AiCallKind::Interlink);
    assert_eq!(call.text, "body");
    assert_eq!(call.page_path, Some("pages/x.md".to_string()));
    assert_eq!(call.scope, AiCallScope::ReplacePage);
}

#[test]
fn research_and_mermaid_prefer_a_selection_then_the_page() {
    let path = "pages/x.md";
    let research = ai_call_for(SlashAction::AiResearch, Some("  topic  "), "body", path)
        .expect("research call");
    assert_eq!(research.kind, AiCallKind::Research);
    assert_eq!(research.text, "topic");
    assert_eq!(research.page_path, None);
    assert_eq!(research.scope, AiCallScope::ReplacePage);

    let research = ai_call_for(SlashAction::AiResearch, None, "body", path).expect("fallback");
    assert_eq!(research.text, "body");

    let mermaid = ai_call_for(SlashAction::AiGenerateMermaid, Some("flow"), "body", path)
        .expect("mermaid call");
    assert_eq!(mermaid.kind, AiCallKind::Mermaid);
    assert_eq!(mermaid.text, "flow");
    assert_eq!(mermaid.scope, AiCallScope::InsertBlockAfterCaret);

    let mermaid =
        ai_call_for(SlashAction::AiGenerateMermaid, None, "body", path).expect("fallback");
    assert_eq!(mermaid.text, "body");
}

#[test]
fn block_actions_and_empty_prompts_produce_no_call() {
    for action in [
        SlashAction::Paragraph,
        SlashAction::Heading1,
        SlashAction::Quote,
        SlashAction::CodeBlock,
        SlashAction::Mermaid,
        SlashAction::MathEquation,
    ] {
        assert!(ai_call_for(action, Some("selection"), "body", "pages/x.md").is_none());
    }
    assert!(ai_call_for(SlashAction::AiSummarizePage, None, "   ", "pages/x.md").is_none());
    assert!(ai_call_for(SlashAction::AiResearch, None, "", "pages/x.md").is_none());
    assert!(ai_call_for(SlashAction::AiGenerateMermaid, None, "", "pages/x.md").is_none());
}

#[test]
fn slash_filter_matches_title_tail_and_aliases() {
    let items = slash_items(false, false);
    let filtered = filter_slash_items(&items, "math");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, SlashAction::MathEquation.id());

    let filtered = filter_slash_items(&items, "web");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, SlashAction::AiResearch.id());

    let filtered = filter_slash_items(&items, "HEADING");
    assert_eq!(filtered.len(), 3);

    assert_eq!(filter_slash_items(&items, "   ").len(), items.len());
}

#[test]
fn marker_menu_has_seven_markers_and_three_priorities() {
    let items = marker_menu_items();
    assert_eq!(items.len(), 10);
    assert_eq!(items.iter().filter(|i| i.marker.is_some()).count(), 7);
    assert_eq!(items.iter().filter(|i| i.priority.is_some()).count(), 3);
    let priority_a = items
        .iter()
        .find(|i| i.id == "priority_a")
        .expect("priority A");
    assert_eq!(priority_a.title_key, "editor.priority_level");
    assert_eq!(
        priority_a.title_vars,
        vec![("priority".to_string(), "A".to_string())]
    );
}

#[test]
fn slash_items_carry_only_translation_keys() {
    for item in slash_items(true, true) {
        assert!(
            item.title_key.starts_with("editor.") || item.title_key.starts_with("aislash_menu.")
        );
        if let Some(subtitle) = item.subtitle_key {
            assert!(subtitle.starts_with("aislash_menu."));
        }
    }
}
