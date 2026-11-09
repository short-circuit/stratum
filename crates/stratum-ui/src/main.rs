//! Stratum gpui shell binary.
//!
//! Opens the native shell at 1280×720. The data source is selected from the
//! environment so the capture harness can render a deterministic vault:
//!
//! - `STRATUM_FIXTURE=1` uses [`stratum_ui::provider::FixtureProvider`].
//! - `STRATUM_VAULT=/path/to/vault` selects the real service-backed provider;
//!   the current directory is used when the variable is absent.
//! - `STRATUM_LOCALE=en|de|el|es|fr|it|ja|zh` selects the UI catalog.
//!
//! Shortcuts are installed from the shared [`stratum_ui::router::SHORTCUTS`]
//! table, with `Modifier::Ctrl` mapped to GPUI's `secondary` modifier.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::*;
use stratum_i18n::{set_locale, Locale};
use stratum_ui::provider::provider_from_env;
use stratum_ui::router::{keystroke_string, ShortcutAction, SHORTCUTS};
use stratum_ui::{init, Shell, StratumAssets, ThemeConfig};

fn main() {
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
                cx.new(|_| Shell::new(Arc::clone(&provider)))
            }) {
                Ok(_) => log::info!("Stratum shell opened (fixture: {fixture})"),
                Err(error) => log::error!("failed to open the Stratum shell: {error}"),
            }
        });
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
