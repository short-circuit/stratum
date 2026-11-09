//! Round-trip and canonicalization integration tests.
//!
//! Canonical fixtures must survive [`canonicalize_body`] byte-for-byte. The
//! non-canonical corpus documents the deliberate normalizations (id injection,
//! property ordering): canonicalization is idempotent, but the first pass is not
//! byte-identical to arbitrary input.
//!
//! The DTO fixtures mirror `src/components/OutlinerEditor/dtoConverters.test.ts`.
//! At this Rust seam the conversion is lossless, including the `!` on file-block
//! asset references; the React `file` block intentionally drops that `!` above
//! this layer, which is asserted in the TS suite rather than hidden here.

use stratum_editor::{
    block_to_dto, blocks_to_dto, canonicalize_body, dto_to_blocks, BlockDto, EditorDocument,
};

const CANONICAL: &[&str] = &[
    include_str!("fixtures/canonical_basic.md"),
    include_str!("fixtures/canonical_nested.md"),
    include_str!("fixtures/canonical_code.md"),
    include_str!("fixtures/canonical_inline.md"),
];

const NON_CANONICAL: &[&str] = &[
    include_str!("fixtures/plain_markdown.md"),
    include_str!("fixtures/plain_no_blocks.md"),
];

#[test]
fn canonical_fixtures_round_trip_byte_identically() {
    for fixture in CANONICAL {
        assert_eq!(
            canonicalize_body(fixture),
            *fixture,
            "canonical fixture changed:\n{fixture}"
        );
    }
}

#[test]
fn non_canonical_corpus_canonicalizes_idempotently() {
    for raw in NON_CANONICAL {
        let once = canonicalize_body(raw);
        let twice = canonicalize_body(&once);
        assert_eq!(
            once, twice,
            "canonicalization is not idempotent for:\n{raw}"
        );
        assert!(once.contains(".id: "), "canonical output must carry ids");
        assert!(
            once.lines().any(|line| line.starts_with("- ")),
            "canonical output must use block syntax"
        );
    }
}

#[test]
fn canonical_fixtures_survive_a_second_pass() {
    for fixture in CANONICAL {
        let once = canonicalize_body(fixture);
        assert_eq!(canonicalize_body(&once), once);
    }
}

fn dto(
    id: &str,
    content: &str,
    marker: Option<&str>,
    priority: Option<&str>,
    properties: Vec<(&str, &str)>,
    heading: Option<u8>,
) -> BlockDto {
    BlockDto {
        id: id.into(),
        content: content.into(),
        parent_id: None,
        left_id: None,
        properties: properties
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        marker: marker.map(str::to_string),
        priority: priority.map(str::to_string),
        collapsed: false,
        heading_level: heading,
    }
}

#[test]
fn dto_round_trip_preserves_every_field() {
    let dtos = vec![
        dto(
            "11111111-1111-4111-8111-111111111111",
            "```mermaid\ngraph LR;\n  X-->Y;\n```",
            Some("DONE"),
            Some("C"),
            vec![("p", "1")],
            None,
        ),
        dto(
            "22222222-2222-4222-8222-222222222222",
            "```js\nconst x = 1;\n```",
            Some("LATER"),
            Some("A"),
            Vec::new(),
            Some(2),
        ),
        dto(
            "33333333-3333-4333-8333-333333333333",
            "![[assets/foo.png]]",
            Some("NOW"),
            Some("B"),
            vec![("alt", "desc")],
            None,
        ),
        dto(
            "44444444-4444-4444-8444-444444444444",
            "![[assets/bar.pdf]]",
            None,
            Some("A"),
            vec![("size", "12")],
            None,
        ),
    ];
    let blocks = dto_to_blocks(dtos.clone());
    let back = blocks_to_dto(&blocks);
    assert_eq!(back.len(), dtos.len());
    for (expected, got) in dtos.iter().zip(back.iter()) {
        assert_eq!(got.id, expected.id);
        assert_eq!(got.content, expected.content);
        assert_eq!(got.marker, expected.marker);
        assert_eq!(got.priority, expected.priority);
        assert_eq!(got.properties, expected.properties);
        assert_eq!(got.heading_level, expected.heading_level);
    }
}

#[test]
fn dto_conversion_keeps_asset_embeds_lossless_at_this_seam() {
    let source = dto(
        "44444444-4444-4444-8444-444444444444",
        "![[assets/bar.pdf]]",
        None,
        None,
        Vec::new(),
        None,
    );
    let block = dto_to_blocks(vec![source.clone()]).remove(0);
    assert_eq!(block_to_dto(&block), source);
}

#[test]
fn dto_round_trip_preserves_parent_and_left_links() {
    let parent = dto(
        "55555555-5555-4555-8555-555555555555",
        "parent",
        None,
        None,
        Vec::new(),
        None,
    );
    let mut child = dto(
        "66666666-6666-4666-8666-666666666666",
        "child",
        Some("TODO"),
        None,
        Vec::new(),
        None,
    );
    child.parent_id = Some("55555555-5555-4555-8555-555555555555".into());
    child.left_id = Some("77777777-7777-4777-8777-777777777777".into());
    let blocks = dto_to_blocks(vec![parent, child]);
    let back = blocks_to_dto(&blocks);
    assert_eq!(
        back[1].parent_id.as_deref(),
        Some("55555555-5555-4555-8555-555555555555")
    );
    assert_eq!(
        back[1].left_id.as_deref(),
        Some("77777777-7777-4777-8777-777777777777")
    );
}

/// Two root blocks without `left_id` must keep their input order and serialize
/// byte-identically on every rebuild. Before the `BlockTree` insertion-order
/// fix, `ordered_ids()`/`body()` followed `HashMap` iteration order instead.
#[test]
fn root_blocks_without_left_links_keep_input_order_across_rebuilds() {
    let first = dto(
        "11111111-1111-4111-8111-111111111111",
        "first root",
        None,
        None,
        Vec::new(),
        None,
    );
    let second = dto(
        "22222222-2222-4222-8222-222222222222",
        "second root",
        None,
        None,
        Vec::new(),
        None,
    );
    let expected_ids = vec![first.id.clone(), second.id.clone()];

    let mut baseline_body: Option<String> = None;
    for iteration in 0..20 {
        let doc = EditorDocument::from_blocks(
            "pages/x.md",
            dto_to_blocks(vec![first.clone(), second.clone()]),
        )
        .unwrap();

        let ids: Vec<String> = doc.ordered_ids().iter().map(|id| id.to_string()).collect();
        assert_eq!(ids, expected_ids, "ordered_ids diverged at {iteration}");

        let body = doc.body();
        match &baseline_body {
            Some(previous) => assert_eq!(&body, previous, "body diverged at {iteration}"),
            None => baseline_body = Some(body),
        }
    }
}
