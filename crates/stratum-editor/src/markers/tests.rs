use chrono::{Duration, TimeZone, Utc};
use pkm_block::{Block, BlockTree, TaskMarker};
use uuid::Uuid;

use super::*;
use crate::document::EditorDocument;

fn block(id: Uuid, content: &str) -> Block {
    Block::new(id, content.to_string())
}

#[test]
fn marker_keywords_are_in_shipped_order() {
    assert_eq!(
        MARKER_KEYWORDS,
        [
            "TODO",
            "DOING",
            "DONE",
            "NOW",
            "LATER",
            "WAITING",
            "CANCELLED"
        ]
    );
    assert_eq!(MARKER_OPTIONS.len(), 7);
    assert_eq!(PRIORITY_OPTIONS.len(), 3);
}

#[test]
fn detects_and_strips_journal_markers() {
    let id = Uuid::new_v4();
    let mut tree = BlockTree::new();
    tree.insert(block(id, "TODO buy milk"));
    detect_and_apply_markers(&mut tree, "journals/2026-11-09.md");
    let b = tree.get(id).unwrap();
    assert_eq!(b.marker, Some(TaskMarker::Todo));
    assert_eq!(b.content, "buy milk");
}

#[test]
fn ignores_non_journal_pages() {
    let id = Uuid::new_v4();
    let mut tree = BlockTree::new();
    tree.insert(block(id, "TODO buy milk"));
    detect_and_apply_markers(&mut tree, "pages/notes.md");
    let b = tree.get(id).unwrap();
    assert_eq!(b.marker, None);
    assert_eq!(b.content, "TODO buy milk");
}

#[test]
fn skips_blocks_that_already_have_a_marker() {
    let id = Uuid::new_v4();
    let mut tree = BlockTree::new();
    tree.insert(block(id, "TODO buy milk").with_marker(TaskMarker::Done));
    detect_and_apply_markers(&mut tree, "journals/2026-11-09.md");
    let b = tree.get(id).unwrap();
    assert_eq!(b.marker, Some(TaskMarker::Done));
    assert_eq!(b.content, "TODO buy milk");
}

#[test]
fn keyword_only_content_becomes_empty() {
    let id = Uuid::new_v4();
    let mut tree = BlockTree::new();
    tree.insert(block(id, "LATER"));
    detect_and_apply_markers(&mut tree, "journals/2026-11-09.md");
    let b = tree.get(id).unwrap();
    assert_eq!(b.marker, Some(TaskMarker::Later));
    assert_eq!(b.content, "");
}

#[test]
fn children_are_not_scanned() {
    let parent = Uuid::new_v4();
    let child = Uuid::new_v4();
    let mut tree = BlockTree::new();
    let mut child_block = block(child, "DONE nested");
    child_block.parent_id = Some(parent);
    tree.insert(block(parent, "parent"));
    tree.insert(child_block);
    detect_and_apply_markers(&mut tree, "journals/2026-11-09.md");
    assert_eq!(tree.get(parent).unwrap().marker, None);
    assert_eq!(tree.get(child).unwrap().marker, None);
    assert_eq!(tree.get(child).unwrap().content, "DONE nested");
}

#[test]
fn badge_row_is_in_document_order() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut first = block(a, "first");
    first.marker = Some(TaskMarker::Todo);
    let mut second = block(b, "second");
    second.left_id = Some(a);
    second.priority = Some(pkm_block::Priority::A);
    let doc = EditorDocument::from_blocks("pages/x.md", vec![first, second]).unwrap();
    let badges = badge_row(&doc);
    assert_eq!(badges.len(), 2);
    assert_eq!(badges[0].block_id, a);
    assert_eq!(badges[1].block_id, b);
}

#[test]
fn reminder_chips_humanize_thresholds() {
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
    let make_doc = |due: chrono::DateTime<Utc>| {
        let id = Uuid::new_v4();
        let b = block(id, "reminder").with_property("due", due.to_rfc3339());
        EditorDocument::from_blocks("pages/x.md", vec![b]).unwrap()
    };

    let chip = &reminder_chips(&make_doc(now + Duration::minutes(5)), now)[0];
    assert_eq!(chip.label_key, "reminders.humanize.due_in_min");
    assert_eq!(chip.vars, vec![("minutes".to_string(), "5".to_string())]);
    assert!(!chip.overdue);

    let chip = &reminder_chips(&make_doc(now - Duration::hours(2)), now)[0];
    assert_eq!(chip.label_key, "reminders.humanize.overdue_hours");
    assert_eq!(chip.vars, vec![("hours".to_string(), "2".to_string())]);
    assert!(chip.overdue);

    let chip = &reminder_chips(&make_doc(now + Duration::seconds(10)), now)[0];
    assert_eq!(chip.label_key, "reminders.humanize.now");

    let chip = &reminder_chips(&make_doc(now - Duration::seconds(10)), now)[0];
    assert_eq!(chip.label_key, "reminders.humanize.overdue_now");

    let chip = &reminder_chips(&make_doc(now + Duration::hours(30)), now)[0];
    assert_eq!(chip.label_key, "reminders.humanize.due_date");
    assert_eq!(chip.vars[0].0, "date");
    assert!(!chip.vars[0].1.is_empty());
}

#[test]
fn set_reminder_writes_and_removes_the_due_property() {
    let id = Uuid::new_v4();
    let mut tree = BlockTree::new();
    tree.insert(block(id, "task"));
    set_reminder(&mut tree, id, Some("2026-11-09T10:00:00+00:00")).unwrap();
    assert_eq!(
        tree.get(id)
            .unwrap()
            .properties
            .get("due")
            .map(String::as_str),
        Some("2026-11-09T10:00:00+00:00")
    );
    set_reminder(&mut tree, id, None).unwrap();
    assert!(!tree.get(id).unwrap().properties.contains_key("due"));
}
