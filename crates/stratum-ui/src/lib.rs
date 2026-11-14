//! Stratum gpui shell.
//!
//! This crate is the native desktop front end for Stratum. It reproduces the
//! shipped React shell (sidebar, navigation, recents, footer, error bar and
//! toaster) on top of [`stratum_app`], while the frozen React
//! application in `src/` remains the reference implementation.
//!
//! The crate is split so the parity contract is testable without opening a
//! window:
//!
//! - [`theme`] ports the shipped palette derivation and maps it onto gpui-kit.
//! - [`router`] owns the route, navigation and shortcut tables.
//! - [`provider`] is the data seam between the shell and the service layer.
//! - [`shell`] renders the chrome; [`panels`] renders the panel stub.
//! - [`assets`] embeds the icons and Inter faces the shell needs.
//!
//! [`init`] installs the theme and fonts. Call [`gpui_kit::init`] before it and
//! open the window with [`gpui_kit::open_window`] so the shell is wrapped in
//! gpui-kit's [`gpui_kit::component::Root`].
//!
//! ```no_run
//! use stratum_ui::{Shell, StratumAssets, ThemeConfig, init};
//! use stratum_ui::provider::FixtureProvider;
//! use gpui_kit::AppContext;
//! use std::sync::Arc;
//!
//! gpui_kit::application()
//!     .with_assets(StratumAssets::new())
//!     .run(|cx| {
//!         gpui_kit::init(cx);
//!         init(cx, &ThemeConfig::default());
//!         let provider = Arc::new(FixtureProvider::new());
//!         let _ = gpui_kit::open_window(
//!             gpui_kit::WindowOptions::default(),
//!             cx,
//!             |_, cx| cx.new(|_| Shell::new(provider)),
//!         );
//!     });
//! ```

/// Embedded shell assets, icon catalog and font registration.
pub mod assets;
/// Mobile shell layout: safe-area insets, bottom navigation and panel variants.
pub mod mobile;
/// Panel stubs hosted by the shell.
pub mod panels;
/// OS glue bridge: tray, global shortcut, notifications, single instance, deep links.
pub mod platform;
/// Data provider seam between the shell and the headless service layer.
pub mod provider;
/// Route, navigation and shortcut tables.
pub mod router;
/// Shell chrome (sidebar, header, footer, error bar, toaster).
pub mod shell;
/// Shipped palette derivation mapped onto gpui-kit tokens.
pub mod theme;

pub use assets::StratumAssets;
pub use mobile::{MobileServices, NoopServices, MOBILE_I18N_KEYS};
pub use panels::panel_view;
pub use provider::{
    derive_recents, provider_from_env, AppProvider, FixtureProvider, RecentPage, ShellData,
    ShellSnapshot, VaultStats,
};
pub use router::{
    keystroke_string, parse_path, parse_target, path_for, percent_decode, percent_encode, Modifier,
    NavItem, Panel, Route, RouteTarget, Shortcut, ShortcutAction, NAV_ITEMS, ROUTES, SHORTCUTS,
};
pub use shell::{
    Onboarding, OnboardingStep, PanelStarts, Shell, ONBOARDING_I18N_KEYS, SHELL_COLLAPSED_WIDTH,
    SHELL_I18N_KEYS, SHELL_WIDTH,
};
pub use theme::{apply_to_kit, ThemeConfig};

use gpui_kit::*;

actions!(
    stratum,
    [ToggleReadMode, DismissOverlay, ToggleSidebar, MobileBack]
);

/// Install the shell theme and font faces.
///
/// `cfg` carries the primary/secondary colours, light/dark mode and optional
/// base font size. Font registration failures are logged rather than fatal:
/// the shell falls back to the platform's default family so the window still
/// opens if a face is unavailable.
pub fn init(cx: &mut gpui::App, cfg: &ThemeConfig) {
    // Fonts must be registered before the theme is applied: the theme resolves
    // its font family against the text system, and an unregistered "Inter"
    // silently falls back to the platform UI font.
    if let Err(error) = assets::register_fonts(cx) {
        log::error!("failed to register Inter: {error}");
    }
    theme::apply_to_kit(cx, cfg);
}
