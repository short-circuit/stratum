//! Deterministic panel data for the fixture provider.
//!
//! The fixture backs tests and the capture harness, so every panel surface
//! returns a stable shape: three search hits, a 2x3 query table, two seeded
//! saved queries, three templates, three cards, five reminders bucketed around
//! the local clock, two plugins and two catalogs. Mutating calls update the
//! state behind a mutex so UI flows can be exercised without a vault.
//!
//! The module is split so every file stays under the repository length limit:
//! [`data`] owns the immutable seed data, [`whiteboards`] owns the board seed
//! data and state helpers, [`impls`] owns the [`PanelData`] implementation and
//! this file owns the mutable state and shared helpers.

mod data;
mod impls;
mod whiteboards;

use std::collections::BTreeMap;
use std::sync::MutexGuard;

use super::FixtureProvider;

/// Mutable panel state owned by the fixture provider.
pub(super) struct PanelState {
    /// Saved queries, seeded with two rows.
    pub saved_queries: Vec<crate::provider::panels::SavedQueryRow>,
    /// Recorded SM-2 review qualities `(card id, quality)`.
    pub reviewed: Vec<(String, u8)>,
    /// Reminder rows bucketed relative to the fixture clock.
    pub reminders: Vec<crate::provider::panels::ReminderRow>,
    /// Installed plugins.
    pub plugins: Vec<crate::provider::panels::PluginRow>,
    /// Installed themes (empty until one is installed).
    pub installed_themes: Vec<crate::provider::panels::InstalledThemeRow>,
    /// Persisted graph settings (mutable so the save round-trip is observable).
    pub graph_settings: crate::provider::panels::GraphSettingsRow,
    /// Whiteboard rows, seeded with three deterministic boards.
    pub whiteboards: Vec<crate::provider::panels::WhiteboardRow>,
    /// Personal whiteboard library items.
    pub whiteboard_library: Vec<crate::provider::panels::LibraryItemRow>,
    /// Exported whiteboard bytes keyed by vault-relative path.
    pub whiteboard_exports: BTreeMap<String, Vec<u8>>,
}

impl PanelState {
    /// Build the deterministic panel state.
    pub(super) fn new() -> Self {
        let now = chrono::Local::now();
        Self {
            saved_queries: data::fixture_saved_queries(),
            reviewed: Vec::new(),
            reminders: data::fixture_reminders(now),
            plugins: data::fixture_plugins(),
            installed_themes: Vec::new(),
            graph_settings: crate::provider::panels::GraphSettingsRow::default(),
            whiteboards: whiteboards::fixture_whiteboards(),
            whiteboard_library: whiteboards::fixture_whiteboard_library(),
            whiteboard_exports: BTreeMap::new(),
        }
    }
}

/// Lock the fixture panel state.
pub(super) fn panel_state(
    provider: &FixtureProvider,
) -> Result<MutexGuard<'_, PanelState>, String> {
    provider
        .panels
        .lock()
        .map_err(|_| "fixture provider lock poisoned".to_string())
}

impl FixtureProvider {
    /// Qualities recorded by [`crate::provider::panels::PanelData::review_card`],
    /// in call order.
    pub fn reviewed_qualities(&self) -> Vec<(String, u8)> {
        panel_state(self)
            .map(|state| state.reviewed.clone())
            .unwrap_or_default()
    }

    /// Add a fixture plugin row (idempotent by id).
    pub(super) fn install_plugin_row(
        &self,
        id: String,
    ) -> Result<crate::provider::panels::PluginRow, String> {
        let mut state = panel_state(self)?;
        if let Some(existing) = state.plugins.iter().find(|plugin| plugin.id == id) {
            return Ok(existing.clone());
        }
        let row = data::fixture_installed_plugin(&id);
        state.plugins.push(row.clone());
        Ok(row)
    }
}
