//! Headless interaction tests: nested click controls must not bubble into the
//! parent handler, and starting the journal on a past day scrolls that day into
//! view. These open real (headless) gpui windows through gpui-kit's test support.

use std::ops::Deref;
use std::sync::Arc;

// Import the Kit explicitly rather than globbing it: the glob also re-exports
// GPUI's `test` macro, which would shadow the built-in `#[test]` that
// `#[gpui_kit::test]` emits and recurse forever.
use gpui_kit::test::TestWindowExt;
use gpui_kit::{px, size, Modifiers, TestAppContext, VisualTestContext};
use stratum_ui::panels::journal::JournalView;
use stratum_ui::panels::page::{PageStart, PageView};
use stratum_ui::panels::query::{QueryStart, QueryView};
use stratum_ui::panels::search::model::SearchStatus;
use stratum_ui::panels::search::{SearchStart, SearchView};
use stratum_ui::panels::voice::{VoiceCapture, VoiceStage};
use stratum_ui::panels::PanelRequests;
use stratum_ui::provider::{journal_path, FixtureProvider, ShellData, FIXTURE_PAGE, FIXTURE_TODAY};
use stratum_ui::ThemeConfig;

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        stratum_ui::init(cx, &ThemeConfig::default());
    });
}

#[gpui_kit::test]
fn backlink_snippet_chip_does_not_navigate(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let start = PageStart {
        cards_open: true,
        ..PageStart::default()
    };
    let (page, cx) =
        cx.add_window_view(move |_, cx| PageView::new(provider, FIXTURE_PAGE, start, cx));
    cx.update(|window, cx| window.render_frame(cx));
    let bounds = cx
        .debug_bounds("page-snippet-chip")
        .expect("snippet chip rendered");
    cx.simulate_click(bounds.center(), Modifiers::default());
    let (snippet, navigation) = cx.update(|_window, cx| {
        page.update(cx, |view, _| {
            (view.snippet().is_some(), view.take_pending_navigation())
        })
    });
    assert!(snippet, "the snippet card must load");
    assert_eq!(navigation, None, "the chip click must not navigate");
}

#[gpui_kit::test]
fn voice_dismiss_chip_does_not_restart_recording(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let (voice, cx) = cx
        .add_window_view(move |_, cx| VoiceCapture::new(journal_path(FIXTURE_TODAY), provider, cx));
    cx.update(|_window, cx| voice.update(cx, |view, cx| view.start(cx)));
    cx.run_until_parked();
    cx.update(|_window, cx| voice.update(cx, |view, cx| view.stop(cx)));
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    assert_eq!(
        cx.update(|_window, cx| voice.read(cx).stage()),
        VoiceStage::Inserted
    );
    let bounds = cx
        .debug_bounds("voice-dismiss")
        .expect("dismiss chip rendered");
    cx.simulate_click(bounds.center(), Modifiers::default());
    assert_eq!(
        cx.update(|_window, cx| voice.read(cx).stage()),
        VoiceStage::Idle,
        "dismiss must not start a new recording"
    );
}

#[gpui_kit::test]
fn journal_start_scrolls_to_the_selected_day(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    // Use a past fixture day that is never the real "today", so the plan always
    // contains it as the first stacked section.
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let target = ["2026-11-09", "2026-11-08", "2026-11-07"]
        .iter()
        .find(|date| **date != today)
        .expect("a past fixture day")
        .to_string();
    let start = target.clone();
    // A deliberately short window keeps the stacked days taller than the
    // viewport, so centring the selected day must produce a real scroll offset.
    let window = cx.open_window(size(px(420.0), px(260.0)), move |_, cx| {
        JournalView::new(provider, Some(start), false, false, cx)
    });
    let journal = window.root(cx).expect("journal window root");
    let cx = VisualTestContext::from_window(*window.deref(), cx).into_mut();
    for _ in 0..3 {
        cx.update(|window, cx| window.render_frame(cx));
        cx.run_until_parked();
    }
    let (offset, date, plan_index) = cx.update(|_window, cx| {
        let view = journal.read(cx);
        (
            view.scroll_offset(),
            view.section_date(0).map(str::to_string),
            view.plan().index_of(&target),
        )
    });
    // Today occupies plan slot 0; the selected past day must be the first
    // stacked past section and resolve to plan slot 1.
    assert_eq!(plan_index, Some(1));
    assert_eq!(date.as_deref(), Some(target.as_str()));
    assert!(
        offset > 0.0,
        "journal must scroll the selected day into view, got {offset}"
    );
}

#[gpui_kit::test]
fn search_result_click_navigates(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let start = SearchStart {
        query: Some("todo".to_string()),
    };
    let (search, cx) =
        cx.add_window_view(move |window, cx| SearchView::new(provider, start, window, cx));
    cx.update(|window, cx| window.render_frame(cx));
    let bounds = cx
        .debug_bounds("search-result-0")
        .expect("search result row rendered");
    cx.simulate_click(bounds.center(), Modifiers::default());
    let navigation = cx.update(|_window, cx| {
        search.update(cx, |view, _| {
            let pending = view.pending_navigation().map(str::to_string);
            view.clear_pending_navigation();
            pending
        })
    });
    assert_eq!(
        navigation.as_deref(),
        Some("pages/fixture.md"),
        "clicking a result must request navigation to its page"
    );
}

#[gpui_kit::test]
fn search_tag_mode_uses_tag_search(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let start = SearchStart {
        query: Some("#project".to_string()),
    };
    let (search, cx) =
        cx.add_window_view(move |window, cx| SearchView::new(provider, start, window, cx));
    cx.update(|window, cx| window.render_frame(cx));
    let (paths, status) = cx.update(|_window, cx| {
        let view = search.read(cx);
        (
            view.results()
                .iter()
                .map(|hit| hit.page_path.clone())
                .collect::<Vec<_>>(),
            view.status().clone(),
        )
    });
    assert_eq!(paths, vec!["pages/fixture.md", "pages/Project Notes.md"]);
    assert_eq!(status, SearchStatus::Done);
}

#[gpui_kit::test]
fn query_run_renders_results(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let (query, cx) = cx.add_window_view(move |window, cx| {
        QueryView::new(provider, QueryStart::default(), window, cx)
    });
    cx.update(|_window, cx| query.update(cx, |view, cx| view.run(cx)));
    cx.update(|window, cx| window.render_frame(cx));
    let (columns, rows, rendered) = cx.update(|_window, cx| {
        let view = query.read(cx);
        let result = view.result().expect("query result present");
        (
            result.columns.clone(),
            result.rows.len(),
            view.error().map(str::to_string),
        )
    });
    assert_eq!(columns, vec!["?b", "?content"]);
    assert_eq!(rows, 3);
    assert_eq!(rendered, None);
    assert!(cx.debug_bounds("query-results").is_some());
}

#[gpui_kit::test]
fn query_save_dialog_adds_saved_query(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let (query, cx) = cx.add_window_view(move |window, cx| {
        QueryView::new(provider, QueryStart::default(), window, cx)
    });
    cx.update(|window, cx| window.render_frame(cx));
    let before = cx.update(|_window, cx| query.read(cx).saved().len());
    cx.update(|_window, cx| query.update(cx, |view, cx| view.open_save_dialog(cx)));
    cx.update(|_window, cx| {
        query.update(cx, |view, cx| view.set_dialog_name("Saved from test", cx))
    });
    cx.update(|_window, cx| query.update(cx, |view, cx| view.submit_dialog(cx)));
    let (after, toast) = cx.update(|_window, cx| {
        let view = query.read(cx);
        (view.saved().len(), view.pending_toast().map(str::to_string))
    });
    assert_eq!(after, before + 1, "saving must append the query");
    assert!(toast.is_some(), "saving must request a success toast");
}
