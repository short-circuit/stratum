//! Stratum gpui shell binary.
//!
//! Opens the native shell at 1280×720. The data source is selected from the
//! environment so the capture harness can render a deterministic vault:
//!
//! - `STRATUM_FIXTURE=1` uses [`stratum_ui::provider::FixtureProvider`].
//! - `STRATUM_VAULT=/path/to/vault` selects the real service-backed provider;
//!   the current directory is used when the variable is absent.
//! - `STRATUM_LOCALE=en|de|el|es|fr|it|ja|zh` selects the UI catalog.
//! - `STRATUM_ROUTE=/journal` opens a route (parsed through the shared router).
//! - `STRATUM_PAGE=pages/Welcome.md` opens a page in the editor.
//! - `STRATUM_JOURNAL_DATE=2026-11-09` starts the journal on a day.
//! - `STRATUM_CALENDAR=1` starts with the journal calendar expanded.
//! - `STRATUM_READ_MODE=1` starts in read mode.
//! - `STRATUM_MENU=note` starts with the page Note menu open.
//! - `STRATUM_CARDS=open` starts with the page backlinks/suggestions expanded.
//! - `STRATUM_HIGHLIGHT=<block-id>` reveals a block when the page opens.
//! - `STRATUM_SNIPPET=1` preloads the first linked backlink's snippet card.
//! - `STRATUM_SEARCH=<query>` submits a search query on the search panel.
//! - `STRATUM_QUERY=<datalog>` prefills the query panel's Datalog editor.
//! - `STRATUM_QUERY_TAB=builder|datalog` selects the query panel tab.
//! - `STRATUM_QUERY_RUN=1` runs the query after construction.
//! - `STRATUM_ASK=<question>` asks the question on the Ask Notes panel.
//! - `STRATUM_FLASHCARD_REVEAL=1` reveals the first flashcard's answer.
//!
//! Shortcuts are installed from the shared [`stratum_ui::router::SHORTCUTS`]
//! table, with `Modifier::Ctrl` mapped to GPUI's `secondary` modifier.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::*;
use stratum_i18n::{set_locale, Locale};
use stratum_ui::panels::ask::AskStart;
use stratum_ui::panels::flashcards::FlashcardsStart;
use stratum_ui::panels::query::{QueryStart, QueryTab};
use stratum_ui::panels::search::SearchStart;
use stratum_ui::provider::provider_from_env;
use stratum_ui::router::{keystroke_string, parse_path, ShortcutAction, SHORTCUTS};
use stratum_ui::{init, PanelStarts, Shell, StratumAssets, ThemeConfig};

fn main() {
    init_logging();
    gpui_kit::application()
        .with_assets(StratumAssets::new())
        .run(|cx| {
            gpui_kit::init(cx);
            init(cx, &ThemeConfig::default());
            install_locale();
            install_shortcuts(cx);

            let fixture = std::env::var("STRATUM_FIXTURE")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let vault = std::env::var("STRATUM_VAULT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let provider = provider_from_env(vault, fixture);

            let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(1280.0), px(720.0)));
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Stratum".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };

            match gpui_kit::open_window(options, cx, |_window, cx| {
                let mut shell = Shell::new(Arc::clone(&provider));
                apply_start_state(&mut shell);
                cx.new(|_| shell)
            }) {
                Ok(_) => log::info!("Stratum shell opened (fixture: {fixture})"),
                Err(error) => log::error!("failed to open the Stratum shell: {error}"),
            }

            // Keyboard shortcuts only reach the shell once the application owns
            // focus; without this the window maps but never receives keystrokes.
            cx.activate(true);
        });
}

/// Apply the deterministic start-state flags used by the capture harness.
fn apply_start_state(shell: &mut Shell) {
    if env_flag("STRATUM_COLLAPSED") {
        shell.toggle_sidebar();
    }
    if let Ok(route) = std::env::var("STRATUM_ROUTE") {
        match parse_path(&route) {
            Some(panel) => shell.navigate(panel),
            None => log::warn!("ignoring unknown STRATUM_ROUTE={route}"),
        }
    }
    let search = std::env::var("STRATUM_SEARCH")
        .ok()
        .filter(|value| !value.is_empty());
    let query_datalog = std::env::var("STRATUM_QUERY")
        .ok()
        .filter(|value| !value.is_empty());
    let query_tab = std::env::var("STRATUM_QUERY_TAB").ok().and_then(|value| {
        match value.to_ascii_lowercase().as_str() {
            "builder" => Some(QueryTab::Builder),
            "datalog" => Some(QueryTab::Datalog),
            other => {
                log::warn!("ignoring unknown STRATUM_QUERY_TAB={other}");
                None
            }
        }
    });
    let query_run = env_flag("STRATUM_QUERY_RUN");
    let ask_question = std::env::var("STRATUM_ASK")
        .ok()
        .filter(|value| !value.is_empty());
    let flashcard_reveal = env_flag("STRATUM_FLASHCARD_REVEAL");
    if search.is_some()
        || query_datalog.is_some()
        || query_tab.is_some()
        || query_run
        || ask_question.is_some()
        || flashcard_reveal
    {
        shell.set_panel_start(PanelStarts {
            search: SearchStart { query: search },
            query: QueryStart {
                datalog: query_datalog,
                tab: query_tab,
                run: query_run,
            },
            ask: AskStart {
                question: ask_question,
            },
            flashcards: FlashcardsStart {
                reveal: flashcard_reveal,
            },
        });
    }
    if let Ok(page) = std::env::var("STRATUM_PAGE") {
        let highlight = std::env::var("STRATUM_HIGHLIGHT")
            .ok()
            .filter(|value| !value.is_empty());
        match highlight {
            Some(block_id) => shell.open_page_with_highlight(page, block_id),
            None => shell.open_page(page),
        }
    }
    if env_flag("STRATUM_READ_MODE") {
        shell.set_read_mode(true);
    }
    let menu_open = std::env::var("STRATUM_MENU")
        .map(|value| value.eq_ignore_ascii_case("note"))
        .unwrap_or(false);
    let cards_open = std::env::var("STRATUM_CARDS")
        .map(|value| value.eq_ignore_ascii_case("open"))
        .unwrap_or(false);
    shell.set_page_start(menu_open, cards_open, None);
    if env_flag("STRATUM_SNIPPET") {
        shell.set_page_snippet(true);
    }
    let journal_date = std::env::var("STRATUM_JOURNAL_DATE")
        .ok()
        .filter(|value| !value.is_empty());
    shell.set_journal_start(journal_date, env_flag("STRATUM_CALENDAR"));
}

/// Read a boolean environment flag.
fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Minimal stderr logger so startup diagnostics (missing fonts, rejected key
/// bindings, failed window creation) are visible without pulling in a logging
/// crate.
struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn init_logging() {
    let logger: &'static StderrLogger = Box::leak(Box::new(StderrLogger));
    let _ = log::set_logger(logger).map(|()| log::set_max_level(log::LevelFilter::Info));
}

/// Apply `STRATUM_LOCALE` when it names a supported catalog.
fn install_locale() {
    if let Ok(code) = std::env::var("STRATUM_LOCALE") {
        match Locale::from_code(&code) {
            Some(locale) => set_locale(locale),
            None => log::warn!("ignoring unsupported STRATUM_LOCALE={code}"),
        }
    }
}

/// Bind every shortcut in the shared table to its shell action.
fn install_shortcuts(cx: &mut gpui::App) {
    for shortcut in SHORTCUTS {
        let keystroke = keystroke_string(&shortcut);
        let action: Box<dyn gpui::Action> = match shortcut.action {
            ShortcutAction::ToggleReadMode => Box::new(stratum_ui::ToggleReadMode),
            ShortcutAction::DismissOverlay => Box::new(stratum_ui::DismissOverlay),
            ShortcutAction::ToggleSidebar => Box::new(stratum_ui::ToggleSidebar),
        };
        match gpui::KeyBinding::load(&keystroke, action, None, false, None, cx) {
            Ok(binding) => cx.bind_keys([binding]),
            Err(error) => log::warn!("invalid shortcut {keystroke}: {error}"),
        }
    }
}
