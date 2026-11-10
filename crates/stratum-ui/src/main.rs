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
//!
//! Shortcuts are installed from the shared [`stratum_ui::router::SHORTCUTS`]
//! table, with `Modifier::Ctrl` mapped to GPUI's `secondary` modifier.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::*;
use stratum_i18n::{set_locale, Locale};
use stratum_ui::provider::provider_from_env;
use stratum_ui::router::{keystroke_string, parse_path, ShortcutAction, SHORTCUTS};
use stratum_ui::{init, Shell, StratumAssets, ThemeConfig};

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
    if let Ok(page) = std::env::var("STRATUM_PAGE") {
        shell.open_page(page);
    }
    if env_flag("STRATUM_READ_MODE") {
        shell.set_read_mode(true);
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
