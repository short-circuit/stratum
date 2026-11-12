//! Tab bodies for the native Settings panel.
//!
//! [`render`] dispatches to the shipped eight tabs. The AI tab is split across
//! [`ai`], [`ai_discovery`] and [`ai_voice`], and the sync tab across [`sync`]
//! and [`backup`], so every file stays well under the repository length limit.
//!
//! Each body receives a [`Palette`] snapshot instead of borrowing the context
//! for colours, which keeps listener construction and rendering independent.

use gpui_kit::prelude::*;

use super::controls::Palette;
use super::{SettingsTab, SettingsView};

pub(super) mod ai;
pub(super) mod ai_discovery;
pub(super) mod ai_voice;
pub(super) mod backup;
pub(super) mod developer;
pub(super) mod notifications;
pub(super) mod research;
pub(super) mod sync;
pub(super) mod theme;
pub(super) mod tray;
pub(super) mod vault;

/// Render the active tab body.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    match view.tab() {
        SettingsTab::Vault => vault::render(view, palette, cx).into_any_element(),
        SettingsTab::Theme => theme::render(view, palette, cx).into_any_element(),
        SettingsTab::Ai => ai::render(view, palette, cx).into_any_element(),
        SettingsTab::Research => research::render(view, palette, cx).into_any_element(),
        SettingsTab::Developer => developer::render(view, palette, cx).into_any_element(),
        SettingsTab::Sync => sync::render(view, palette, cx).into_any_element(),
        SettingsTab::Tray => tray::render(view, palette, cx).into_any_element(),
        SettingsTab::Notifications => notifications::render(view, palette, cx).into_any_element(),
    }
}
