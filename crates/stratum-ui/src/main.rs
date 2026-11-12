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
//! - `STRATUM_REMINDER_DIALOG=<block-id>` opens that reminder's due dialog.
//! - `STRATUM_PLUGINS_TAB=installed|directory|themes` opens a plugins tab.
//! - `STRATUM_GRAPH_MODE=full|component|orphans` selects the graph view mode.
//! - `STRATUM_GRAPH_COMPONENT=<index>` selects a connected component.
//! - `STRATUM_GRAPH_SETTINGS=1` opens the graph settings panel.
//! - `STRATUM_GRAPH_SEARCH=<text>` prefills the graph node search.
//! - `STRATUM_GRAPH_TICKS=<count>` sets the deterministic layout ticks.
//! - `STRATUM_SETTINGS_TAB=vault|theme|ai|research|developer|sync|tray|notifications`
//!   opens a Settings tab.
//! - `STRATUM_SETTINGS_CONFLICTS=1` seeds the fixture sync status with conflicts.
//! - `STRATUM_SETTINGS_COMMITS=1` expands the Developer tab commit log.
//! - `STRATUM_QUICK_CAPTURE=1` opens the Quick Capture dialog on start.
//! - `STRATUM_QUICK_CAPTURE_VOICE=1` opens it in voice mode.
//! - `STRATUM_DEEP_LINK=stratum://board/Name#element` routes a deep link on
//!   start, exercising the same path a `stratum://` URL takes.
//!
//! The OS glue (tray, global shortcut, notifications, single instance) is
//! started before the window: a second launch forwards its argv to the running
//! instance and exits. Fixture runs use a per-process instance name so a capture
//! never contends with a developer's running app.
//!
//! `STRATUM_REMINDER_TOAST=1` makes the fixture report due reminders, which the
//! shell polls into its reminder toaster.
//!
//! Shortcuts are installed from the shared [`stratum_ui::router::SHORTCUTS`]
//! table, with `Modifier::Ctrl` mapped to GPUI's `secondary` modifier.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::*;
use stratum_i18n::{set_locale, Locale};
use stratum_ui::panels::ask::AskStart;
use stratum_ui::panels::canvas::{tool_from_env, CanvasStart, ExportFormat};
use stratum_ui::panels::flashcards::FlashcardsStart;
use stratum_ui::panels::graph::model::{view_mode_from_env, DEFAULT_TICKS};
use stratum_ui::panels::graph::GraphStart;
use stratum_ui::panels::plugins::{PluginsStart, PluginsTab};
use stratum_ui::panels::query::{QueryStart, QueryTab};
use stratum_ui::panels::reminders::RemindersStart;
use stratum_ui::panels::search::SearchStart;
use stratum_ui::panels::settings::{SettingsStart, SettingsTab};
use stratum_ui::provider::provider_from_env;
use stratum_ui::router::{keystroke_string, parse_target, ShortcutAction, SHORTCUTS};
use stratum_ui::{init, PanelStarts, Shell, StratumAssets, ThemeConfig};

fn main() {
    init_logging();
    gpui_kit::application()
        .with_assets(StratumAssets::new())
        .run(|cx| {
            gpui_kit::init(cx);
            install_locale();

            let fixture = std::env::var("STRATUM_FIXTURE")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let vault = std::env::var("STRATUM_VAULT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let provider = provider_from_env(vault, fixture);
            // Claim the single-instance slot before the window exists: a second
            // launch hands its argv (usually a `stratum://` deep link) to the
            // running instance and exits without opening a second window.
            let instance = if fixture {
                format!(
                    "{}-fixture-{}",
                    stratum_ui::platform::instance_name(),
                    std::process::id()
                )
            } else {
                stratum_ui::platform::instance_name()
            };
            let args: Vec<String> = std::env::args().skip(1).collect();
            let Some(platform) =
                stratum_ui::platform::PlatformBridge::bootstrap_named(&provider, &instance, args)
            else {
                log::info!("another Stratum instance is running; launch arguments forwarded");
                // The forwarded launch must not keep a gpui event loop alive with
                // no window: exit as soon as the argv is handed over.
                std::process::exit(0);
            };
            // The shipped page applies the persisted palette on load; doing it
            // before the first window means the shell never flashes the default
            // theme. Invalid hex values fall back inside `from_settings`.
            let theme = provider
                .settings()
                .map(|settings| ThemeConfig::from_settings(&settings.theme))
                .unwrap_or_default();
            init(cx, &theme);
            install_shortcuts(cx);

            let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(1280.0), px(720.0)));
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Stratum".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };

            match gpui_kit::open_window(options, cx, move |_window, cx| {
                let mut shell = Shell::with_platform(Arc::clone(&provider), platform);
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
        match parse_target(&route) {
            Some(target) => {
                shell.navigate(target.panel);
                if target.panel == stratum_ui::router::Panel::Whiteboards {
                    shell.set_canvas_start(CanvasStart::from_target(&target));
                }
            }
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
    let reminder_dialog = std::env::var("STRATUM_REMINDER_DIALOG")
        .ok()
        .filter(|value| !value.is_empty());
    let plugins_tab = std::env::var("STRATUM_PLUGINS_TAB").ok().and_then(|value| {
        match PluginsTab::from_env(&value) {
            Some(tab) => Some(tab),
            None => {
                log::warn!("ignoring unknown STRATUM_PLUGINS_TAB={value}");
                None
            }
        }
    });
    let graph_mode = std::env::var("STRATUM_GRAPH_MODE").ok().and_then(|value| {
        let mode = view_mode_from_env(&value);
        if mode.is_none() {
            log::warn!("ignoring unknown STRATUM_GRAPH_MODE={value}");
        }
        mode
    });
    let graph_component = std::env::var("STRATUM_GRAPH_COMPONENT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());
    let graph_settings = env_flag("STRATUM_GRAPH_SETTINGS");
    let graph_search = std::env::var("STRATUM_GRAPH_SEARCH")
        .ok()
        .filter(|value| !value.is_empty());
    let graph_ticks = std::env::var("STRATUM_GRAPH_TICKS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());
    let whiteboard_board = std::env::var("STRATUM_WHITEBOARD")
        .ok()
        .filter(|value| !value.is_empty());
    let whiteboard_tool = std::env::var("STRATUM_WHITEBOARD_TOOL")
        .ok()
        .and_then(|value| match tool_from_env(&value) {
            Some(tool) => Some(tool),
            None => {
                log::warn!("ignoring unknown STRATUM_WHITEBOARD_TOOL={value}");
                None
            }
        });
    let whiteboard_library = env_flag("STRATUM_WHITEBOARD_LIBRARY");
    let whiteboard_style = env_flag("STRATUM_WHITEBOARD_STYLE");
    let whiteboard_export = std::env::var("STRATUM_WHITEBOARD_EXPORT")
        .ok()
        .and_then(|value| match ExportFormat::from_env(&value) {
            Some(format) => Some(format),
            None => {
                log::warn!("ignoring unknown STRATUM_WHITEBOARD_EXPORT={value}");
                None
            }
        });
    let whiteboard_element = std::env::var("STRATUM_WHITEBOARD_ELEMENT")
        .ok()
        .filter(|value| !value.is_empty());
    let whiteboard_text = env_flag("STRATUM_WHITEBOARD_TEXT");
    let settings_tab = std::env::var("STRATUM_SETTINGS_TAB")
        .ok()
        .and_then(|value| match SettingsTab::from_env(&value) {
            Some(tab) => Some(tab),
            None => {
                log::warn!("ignoring unknown STRATUM_SETTINGS_TAB={value}");
                None
            }
        });
    let canvas = CanvasStart {
        board: whiteboard_board,
        tool: whiteboard_tool,
        library_open: whiteboard_library,
        style_open: whiteboard_style,
        export: whiteboard_export,
        element: whiteboard_element,
        text_edit: whiteboard_text,
    };
    let canvas_started = canvas != CanvasStart::default();
    if search.is_some()
        || query_datalog.is_some()
        || query_tab.is_some()
        || query_run
        || ask_question.is_some()
        || flashcard_reveal
        || reminder_dialog.is_some()
        || plugins_tab.is_some()
        || graph_mode.is_some()
        || graph_component.is_some()
        || graph_settings
        || graph_search.is_some()
        || graph_ticks.is_some()
        || canvas_started
        || settings_tab.is_some()
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
            reminders: RemindersStart {
                dialog_block: reminder_dialog,
            },
            plugins: PluginsStart { tab: plugins_tab },
            graph: GraphStart {
                view_mode: graph_mode,
                component: graph_component.unwrap_or(0),
                settings_open: graph_settings,
                search: graph_search,
                ticks: graph_ticks.unwrap_or(DEFAULT_TICKS),
            },
            canvas,
            settings: SettingsStart { tab: settings_tab },
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
    // Either flag opens the dialog; the voice flag selects the mode, so
    // `STRATUM_QUICK_CAPTURE_VOICE=1` alone is enough for the capture harness.
    let voice_capture = env_flag("STRATUM_QUICK_CAPTURE_VOICE");
    if voice_capture || env_flag("STRATUM_QUICK_CAPTURE") {
        shell.request_quick_capture(voice_capture);
    }
    if let Ok(link) = std::env::var("STRATUM_DEEP_LINK") {
        match stratum_platform::deep_link::parse(&link) {
            Some(link) => shell.apply_deep_link(&link),
            None => log::warn!("ignoring unknown STRATUM_DEEP_LINK={link}"),
        }
    }
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
