use pkm_block::{Block, BlockId};
use pkm_core::PkmError;
use uuid::Uuid;

use super::*;

const A: &str = "11111111-1111-4111-8111-111111111111";
const B: &str = "22222222-2222-4222-8222-222222222222";
const C: &str = "33333333-3333-4333-8333-333333333333";

#[test]
fn canonicalize_is_idempotent_on_canonical_input() {
    let raw = format!("- hello\n  .id: {A}\n");
    assert_eq!(canonicalize_body(&raw), raw);
}

#[test]
fn canonicalize_adds_ids_and_orders_properties() {
    let raw = "- task\n  .alpha: one\n  .marker: TODO\n  .priority: A\n  .heading-level: 2\n";
    let canonical = canonicalize_body(raw);
    assert!(canonical.contains(".marker: TODO"));
    assert!(canonical.contains(".priority: A"));
    assert!(canonical.contains(".heading-level: 2"));
    assert!(canonical.contains(".alpha: one"));
    assert!(canonical.contains(".id: "));
    assert_eq!(canonicalize_body(&canonical), canonical);
}

#[test]
fn canonicalize_handles_nested_blocks() {
    let raw = format!("- parent\n  .id: {A}\n  - child\n    .id: {B}\n");
    assert_eq!(canonicalize_body(&raw), raw);
}

#[test]
fn open_reports_body_and_revision() {
    let raw = format!("- hello\n  .id: {A}\n");
    let doc = EditorDocument::open("pages/x.md", &raw).unwrap();
    assert_eq!(doc.path(), "pages/x.md");
    assert_eq!(doc.revision(), 0);
    assert!(!doc.is_dirty());
    assert_eq!(doc.body(), raw);
    assert_eq!(doc.ordered_ids().len(), 1);
}

#[test]
fn markdown_preserves_frontmatter() {
    let raw = format!("---\ntitle: Test\n---\n\n- hello\n  .id: {A}\n");
    let doc = EditorDocument::open("pages/x.md", &raw).unwrap();
    let md = doc.markdown();
    assert!(md.contains("title: Test"));
    assert!(md.contains("- hello"));
    assert!(md.contains(&format!(".id: {A}")));
}

#[test]
fn plain_text_omits_frontmatter_properties_and_ids() {
    let raw = format!(
        "---\ntitle: Notes\n---\n\n- task\n  .marker: TODO\n  .priority: A\n  .id: {A}\n  - child\n    .id: {B}\n"
    );
    let doc = EditorDocument::open("pages/x.md", &raw).unwrap();
    let text = doc.plain_text();
    assert!(!text.contains(".id:"), "{text}");
    assert!(!text.contains(".marker:"), "{text}");
    assert!(!text.contains(".priority:"), "{text}");
    assert!(!text.contains("---"), "{text}");
    assert_eq!(text, "task\nchild");
}

#[test]
fn plain_markdown_lossy_preserves_nesting_without_ids() {
    let raw = format!(
        "---\ntitle: Notes\n---\n\n- parent\n  .id: {A}\n  - child\n    .id: {B}\n  - sibling\n    .id: {C}\n"
    );
    let doc = EditorDocument::open("pages/x.md", &raw).unwrap();
    let markdown = doc.plain_markdown_lossy();
    assert!(
        markdown.lines().all(|line| !line.contains(".id:")),
        "{markdown}"
    );
    assert_eq!(markdown, "- parent\n  - child\n  - sibling");
}

#[test]
fn plain_markdown_lossy_renders_headings_without_property_lines() {
    let raw = format!("- Title\n  .heading-level: 2\n  .id: {A}\n");
    let doc = EditorDocument::open("pages/x.md", &raw).unwrap();
    assert_eq!(doc.plain_markdown_lossy(), "## Title");
}

#[test]
fn mutate_bumps_revision_and_dirty() {
    let id = Uuid::parse_str(A).unwrap();
    let block = Block::new(id, "hello".into());
    let mut doc = EditorDocument::from_blocks("pages/x.md", vec![block]).unwrap();
    doc.mutate(|tree| {
        tree.get_mut(id).unwrap().content = "changed".into();
        Ok(())
    })
    .unwrap();
    assert_eq!(doc.revision(), 1);
    assert!(doc.is_dirty());
    doc.mark_clean();
    assert!(!doc.is_dirty());
}

#[test]
fn failed_mutation_leaves_document_clean() {
    let mut doc = EditorDocument::from_blocks("pages/x.md", Vec::new()).unwrap();
    let result: Result<(), PkmError> = doc.mutate(|_| Err(PkmError::Internal("boom".into())));
    assert!(result.is_err());
    assert_eq!(doc.revision(), 0);
    assert!(!doc.is_dirty());
}

#[test]
fn save_snapshot_marks_clean() {
    let mut doc = EditorDocument::from_blocks("pages/x.md", Vec::new()).unwrap();
    let id = Uuid::new_v4();
    doc.mutate(|tree| {
        tree.insert(Block::new(id, "x".into()));
        Ok(())
    })
    .unwrap();
    let snapshot = doc.save_snapshot();
    assert!(snapshot.contains("- x"));
    assert!(!doc.is_dirty());
}

#[test]
fn block_lookup_and_ordered_ids() {
    let id: BlockId = Uuid::parse_str(A).unwrap();
    let doc = EditorDocument::from_blocks("pages/x.md", vec![Block::new(id, "x".into())]).unwrap();
    assert_eq!(doc.block(id).unwrap().content, "x");
    assert_eq!(doc.ordered_ids(), vec![id]);
}
