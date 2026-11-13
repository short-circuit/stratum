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
use stratum_app::reminders::due::local_input_to_utc_iso;
use stratum_ui::panels::ask::{AskStart, AskView};
use stratum_ui::panels::flashcards::{FlashcardsStart, FlashcardsView};
use stratum_ui::panels::journal::JournalView;
use stratum_ui::panels::page::{PageStart, PageView};
use stratum_ui::panels::plugins::{PluginsStart, PluginsTab, PluginsView};
use stratum_ui::panels::query::{QueryStart, QueryView};
use stratum_ui::panels::reminders::{RemindersStart, RemindersView};
use stratum_ui::panels::search::model::SearchStatus;
use stratum_ui::panels::search::{SearchStart, SearchView};
use stratum_ui::panels::templates::TemplatesView;
use stratum_ui::panels::voice::{VoiceCapture, VoiceStage};
use stratum_ui::panels::PanelRequests;
use stratum_ui::provider::{
    journal_path, FixtureProvider, PanelData, ReminderRow, SettingsData, ShellData, ShellSnapshot,
    FIXTURE_JOURNAL_DATES, FIXTURE_PAGE, FIXTURE_TODAY,
};
use stratum_ui::router::Panel;
use stratum_ui::{Shell, ThemeConfig};

/// A provider that reports one due reminder, so the shell toaster runs without
/// depending on the fixture's `STRATUM_REMINDER_TOAST` environment flag.
struct DueProvider;

impl ShellData for DueProvider {
    fn snapshot(&self) -> ShellSnapshot {
        ShellSnapshot {
            vault: None,
            recents: Vec::new(),
        }
    }

    fn refresh(&self) -> ShellSnapshot {
        self.snapshot()
    }

    fn export_html(&self) -> Result<u64, String> {
        Ok(0)
    }
}

impl SettingsData for DueProvider {}

impl PanelData for DueProvider {
    fn due_reminders(&self) -> Vec<ReminderRow> {
        vec![ReminderRow {
            block_id: "reminder-1".to_string(),
            page_path: FIXTURE_PAGE.to_string(),
            page_title: Some("Fixture Note".to_string()),
            content: "Review the fixture outline".to_string(),
            due: "2026-11-10T09:00:00Z".to_string(),
        }]
    }
}

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
    // The fixture seeds 2026-11-09..2026-10-31 and creates its fixed
    // `FIXTURE_TODAY` page on demand, so on a clock later than 2026-11-10 that
    // page is the newest past day. Derive the target from the fixture dates
    // strictly before the real today so it is always the first stacked past
    // section instead of a hard-coded date that drifts with the calendar.
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut candidates = vec![FIXTURE_TODAY];
    candidates.extend(FIXTURE_JOURNAL_DATES);
    let target = candidates
        .into_iter()
        .find(|date| *date < today.as_str())
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

#[gpui_kit::test]
fn ask_question_renders_answer_and_citations(cx: &mut TestAppContext) {
    init(cx);
    let fixture = Arc::new(FixtureProvider::new());
    let provider: Arc<dyn ShellData> = fixture.clone();
    let start = AskStart {
        question: Some("what did I decide about the project?".to_string()),
    };
    let (ask, cx) = cx.add_window_view(move |window, cx| AskView::new(provider, start, window, cx));
    cx.update(|window, cx| window.render_frame(cx));
    let (answer, sources) = cx.update(|_window, cx| {
        let view = ask.read(cx);
        (
            view.answer().map(|answer| answer.answer.clone()),
            view.answer()
                .map(|answer| answer.sources.len())
                .unwrap_or(0),
        )
    });
    assert_eq!(
        answer.as_deref(),
        Some("The notes suggest deciding after the sprint review.")
    );
    assert_eq!(sources, 2);
    assert!(cx.debug_bounds("ask-answer").is_some());
    assert!(cx.debug_bounds("ask-source-0").is_some());
    assert!(cx.debug_bounds("ask-source-1").is_some());

    let bounds = cx
        .debug_bounds("ask-read-aloud")
        .expect("read-aloud control rendered");
    cx.simulate_click(bounds.center(), Modifiers::default());
    assert_eq!(
        fixture.spoken_texts(),
        vec!["The notes suggest deciding after the sprint review.".to_string()]
    );
}

#[gpui_kit::test]
fn templates_apply_pushes_a_created_toast(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let (templates, cx) =
        cx.add_window_view(move |window, cx| TemplatesView::new(provider, window, cx));
    let target = "pages/from-template.md";
    cx.update(|window, cx| templates.update(cx, |view, cx| view.set_target(target, window, cx)));
    cx.update(|window, cx| window.render_frame(cx));
    let bounds = cx
        .debug_bounds("template-apply-0")
        .expect("template apply button rendered");
    cx.simulate_click(bounds.center(), Modifiers::default());
    let (toast, remaining_target) = cx.update(|_window, cx| {
        let view = templates.read(cx);
        (
            view.pending_toast().map(str::to_string),
            view.target_value().to_string(),
        )
    });
    let toast = toast.expect("applying a template must request a toast");
    assert!(
        toast.contains(target),
        "the toast must name the target path, got {toast}"
    );
    assert_eq!(remaining_target, "", "the target input must clear");
}

#[gpui_kit::test]
fn flashcard_reveal_and_rate(cx: &mut TestAppContext) {
    init(cx);
    let fixture = Arc::new(FixtureProvider::new());
    let provider: Arc<dyn ShellData> = fixture.clone();
    let (flashcards, cx) = cx.add_window_view(move |_window, cx| {
        FlashcardsView::new(provider, FlashcardsStart::default(), cx)
    });
    cx.update(|window, cx| window.render_frame(cx));
    let card = cx
        .debug_bounds("flashcard-card")
        .expect("flashcard rendered");
    cx.simulate_click(card.center(), Modifiers::default());
    cx.update(|window, cx| window.render_frame(cx));

    let revealed = cx.update(|_window, cx| flashcards.read(cx).revealed());
    assert!(revealed, "clicking the card must reveal the answer");
    let rating = cx
        .debug_bounds("flashcard-rating-3")
        .expect("the rating row must render after the reveal");
    cx.simulate_click(rating.center(), Modifiers::default());

    let (reviewed, index) = cx.update(|_window, cx| {
        (
            fixture.reviewed_qualities(),
            flashcards.read(cx).session().current_index(),
        )
    });
    assert_eq!(reviewed, vec![("card-1".to_string(), 3)]);
    assert_eq!(index, Some(1), "the session must advance after a rating");
}

#[gpui_kit::test]
fn reminder_row_open_requests_the_block(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let (reminders, cx) = cx.add_window_view(move |window, cx| {
        RemindersView::new(provider, RemindersStart::default(), window, cx)
    });
    cx.update(|window, cx| window.render_frame(cx));
    let bounds = cx
        .debug_bounds("reminder-row-0-open")
        .expect("the first overdue row renders its open chip");
    cx.simulate_click(bounds.center(), Modifiers::default());
    let pending =
        cx.update(|_window, cx| reminders.update(cx, |view, _| view.take_pending_open_block()));
    let (page_path, block_id) = pending.expect("open must request the block");
    assert_eq!(page_path, "pages/fixture.md");
    assert!(
        block_id.ends_with("overdue-1"),
        "the fixture's first reminder must be opened, got {block_id}"
    );
}

#[gpui_kit::test]
fn reminder_dialog_opens_with_the_quick_choices(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let start = RemindersStart {
        dialog_block: Some("00000000-0000-5000-8000-000overdue-1".to_string()),
    };
    let (reminders, cx) =
        cx.add_window_view(move |window, cx| RemindersView::new(provider, start, window, cx));
    cx.update(|window, cx| window.render_frame(cx));
    assert!(
        cx.debug_bounds("reminder-dialog").is_some(),
        "the start flag must open the due dialog"
    );
    assert!(cx.debug_bounds("reminder-dialog-input").is_some());
    let quick = cx
        .debug_bounds("reminder-dialog-quick-0")
        .expect("the quick choices render");
    cx.simulate_click(quick.center(), Modifiers::default());
    let value = cx
        .update(|_window, cx| {
            reminders
                .read(cx)
                .dialog()
                .map(|dialog| dialog.value.clone())
        })
        .expect("the dialog stays open");
    assert!(
        !local_input_to_utc_iso(&value).is_empty(),
        "the quick choice must fill a valid datetime-local value, got {value}"
    );
}

#[gpui_kit::test]
fn reminder_toast_action_opens_the_page(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(DueProvider);
    let (shell, cx) = cx.add_window_view(move |_, _cx| Shell::new(provider));
    cx.update(|window, cx| window.render_frame(cx));
    let bounds = cx
        .debug_bounds("toast-action")
        .expect("the due-reminder toast renders its open action");
    cx.simulate_click(bounds.center(), Modifiers::default());
    let (page, panel, toast) = cx.update(|_window, cx| {
        let view = shell.read(cx);
        (
            view.current_page().map(str::to_string),
            view.active_panel(),
            view.toast_message().map(str::to_string),
        )
    });
    assert_eq!(page.as_deref(), Some(FIXTURE_PAGE));
    assert_eq!(panel, Panel::Page);
    assert_eq!(toast, None, "the open action must dismiss the toast");
}

#[gpui_kit::test]
fn plugins_tab_switch_loads_the_directory(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let (plugins, cx) = cx.add_window_view(move |_window, cx| {
        PluginsView::new(provider, PluginsStart::default(), cx)
    });
    cx.update(|window, cx| window.render_frame(cx));
    assert!(cx.debug_bounds("plugins-installed").is_some());
    let tab = cx
        .debug_bounds("plugins-tab-plugins.tab_directory")
        .expect("the directory tab renders");
    cx.simulate_click(tab.center(), Modifiers::default());
    cx.update(|window, cx| window.render_frame(cx));
    let (tab, entries) = cx.update(|_window, cx| {
        let view = plugins.read(cx);
        (
            view.tab(),
            view.catalog()
                .map(|catalog| catalog.entries.len())
                .unwrap_or(0),
        )
    });
    assert_eq!(tab, PluginsTab::Directory);
    assert_eq!(entries, 3);
    assert!(cx.debug_bounds("plugin-catalog-graph-export").is_some());
}

#[gpui_kit::test]
fn plugin_smoke_test_records_an_inline_result(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let (plugins, cx) = cx.add_window_view(move |_window, cx| {
        PluginsView::new(provider, PluginsStart::default(), cx)
    });
    cx.update(|window, cx| window.render_frame(cx));
    let note = cx
        .debug_bounds("plugin-test-note-word-count")
        .expect("the note-read control renders");
    cx.simulate_click(note.center(), Modifiers::default());
    cx.update(|window, cx| window.render_frame(cx));
    let outcome = cx
        .update(|_window, cx| plugins.read(cx).test_result("word-count").cloned())
        .expect("the smoke test must record a result");
    assert!(outcome.result.ok);
    assert!(cx.debug_bounds("plugin-test-result-word-count").is_some());
}

#[gpui_kit::test]
fn theme_install_applies_the_catalog_entry(cx: &mut TestAppContext) {
    init(cx);
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let start = PluginsStart {
        tab: Some(PluginsTab::Themes),
    };
    let (plugins, cx) =
        cx.add_window_view(move |_window, cx| PluginsView::new(provider, start, cx));
    cx.update(|window, cx| window.render_frame(cx));
    let install = cx
        .debug_bounds("theme-install-midnight")
        .expect("the catalog theme renders");
    cx.simulate_click(install.center(), Modifiers::default());
    cx.update(|window, cx| window.render_frame(cx));
    let installed = cx.update(|_window, cx| plugins.read(cx).installed_themes().to_vec());
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id, "midnight");
    assert!(installed[0].active);
    assert!(cx.debug_bounds("theme-installed-midnight").is_some());
}
