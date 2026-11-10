//! Fixture provider tests: page sizes, assets, journal and voice flows.
//!
//! These run without a window and exercise the deterministic fixture data the
//! capture harness relies on.

use super::*;
use stratum_editor::menus::{AiCallScope, AiTransformAction};

#[test]
fn welcome_size_is_selectable() {
    let provider = FixtureProvider::with_welcome_blocks(10_000);
    let blocks = provider.load_blocks("pages/Welcome.md").expect("page");
    assert_eq!(blocks.len(), 10_000);
}

#[test]
fn fixture_journal_dates_and_today_are_fixed() {
    let provider = FixtureProvider::new();
    assert_eq!(
        provider.journal_dates(),
        vec!["2026-11-09", "2026-11-08", "2026-11-07"]
    );
    assert_eq!(
        provider.ensure_today_journal(),
        Ok(journal_path(FIXTURE_TODAY))
    );
    let blocks = provider
        .load_blocks(&journal_path(FIXTURE_TODAY))
        .expect("today page exists");
    assert_eq!(blocks.len(), 5);
    assert!(blocks[0].content.contains(FIXTURE_TODAY));
    let past = provider
        .load_blocks(&journal_path("2026-11-09"))
        .expect("past page exists");
    assert_ne!(blocks[0].content, past[0].content);
    assert_eq!(
        provider.create_journal_day("2026-11-06"),
        Ok(journal_path("2026-11-06"))
    );
    assert!(provider.journal_dates().contains(&"2026-11-06".to_string()));
    assert_eq!(provider.repair_db(), Ok(()));
}

#[test]
fn fixture_voice_flow_is_deterministic() {
    let provider = FixtureProvider::new();
    assert!(provider.stt_configured());
    provider.ensure_today_journal().expect("today page");
    assert_eq!(
        provider.voice_start(&journal_path(FIXTURE_TODAY)),
        Ok(FIXTURE_RECORDING.to_string())
    );
    assert_eq!(
        provider.voice_stop(),
        Ok((FIXTURE_RECORDING.to_string(), 3.0))
    );
    let before = provider
        .load_blocks(&journal_path(FIXTURE_TODAY))
        .expect("today page")
        .len();
    assert_eq!(
        provider.voice_transcribe(FIXTURE_RECORDING, &journal_path(FIXTURE_TODAY)),
        Ok(2)
    );
    let after = provider
        .load_blocks(&journal_path(FIXTURE_TODAY))
        .expect("today page")
        .len();
    assert_eq!(after, before + 2);
    assert_eq!(
        provider.voice_insert_clip(FIXTURE_RECORDING, &journal_path(FIXTURE_TODAY), 1.0),
        Ok(())
    );
    assert_eq!(
        provider
            .load_blocks(&journal_path(FIXTURE_TODAY))
            .expect("today page")
            .len(),
        after + 1
    );
}

#[test]
fn fixture_asset_round_trip_is_deterministic() {
    let provider = FixtureProvider::new();
    let bytes = b"\x89PNG\r\n\x1a\nfixture".to_vec();
    let first = provider
        .import_asset_bytes(bytes.clone(), "photo.png", Some("image/png"))
        .expect("import");
    let second = provider
        .import_asset_bytes(bytes, "photo.png", None)
        .expect("import");
    assert_eq!(first, second, "same bytes and name must dedupe");
    assert!(first.ref_path.starts_with("assets/photo-"));
    assert!(first.ref_path.ends_with(".png"));
    assert_eq!(
        first.ref_path.len(),
        "assets/photo-".len() + 12 + ".png".len()
    );
    assert_eq!(first.mime, "image/png");
    assert_eq!(first.size, 15);
    assert_eq!(first.size_label, "15 B");

    // The attachment is only listed once the page content references it.
    assert!(provider.page_attachments("pages/Welcome.md").is_empty());
    let blocks = vec![BlockDto {
        id: Uuid::new_v4().to_string(),
        content: format!("![[{}]]", first.ref_path),
        parent_id: None,
        left_id: None,
        properties: Vec::new(),
        marker: None,
        priority: None,
        collapsed: false,
        heading_level: None,
    }];
    provider
        .save_blocks("pages/Photo.md", blocks)
        .expect("save page");
    assert_eq!(
        provider.page_attachments("pages/Photo.md"),
        vec![first.clone()]
    );

    provider.delete_asset(&first.ref_path).expect("delete");
    assert!(provider.page_attachments("pages/Photo.md").is_empty());
    assert_eq!(provider.delete_asset(&first.ref_path), Ok(()));
}

#[test]
fn fixture_import_path_reads_the_file() {
    let provider = FixtureProvider::new();
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("Report.pdf");
    std::fs::write(&path, b"%PDF-1.7 fixture").expect("write file");
    let row = provider
        .import_asset_path(path.to_str().expect("utf-8 path"))
        .expect("import");
    assert_eq!(row.mime, "application/pdf");
    assert!(row.ref_path.starts_with("assets/report-"));
    assert!(row.ref_path.ends_with(".pdf"));
}

#[test]
fn fixture_ai_run_is_deterministic() {
    let provider = FixtureProvider::new();
    let transform = AiCall {
        kind: AiCallKind::Transform(AiTransformAction::Rewrite),
        scope: AiCallScope::InsertAtCaret,
        text: "hello".into(),
        page_path: None,
    };
    assert_eq!(provider.ai_run(transform), Ok("hello".to_string()));
    let research = AiCall {
        kind: AiCallKind::Research,
        scope: AiCallScope::ReplacePage,
        text: "topic".into(),
        page_path: None,
    };
    assert_eq!(provider.ai_run(research), Ok("topic".to_string()));
    let interlink = AiCall {
        kind: AiCallKind::Interlink,
        scope: AiCallScope::ReplacePage,
        text: "body".into(),
        page_path: Some("pages/x.md".into()),
    };
    assert_eq!(provider.ai_run(interlink), Ok("body".to_string()));
    let mermaid = AiCall {
        kind: AiCallKind::Mermaid,
        scope: AiCallScope::InsertBlockAfterCaret,
        text: "prompt".into(),
        page_path: None,
    };
    assert_eq!(
        provider.ai_run(mermaid),
        Ok("graph TD\n  A[Start] --> B[End]".to_string())
    );
}
