//! Window-free unit tests for the native graph panel entity and gate list.

use ::core::prelude::v1::test;

use super::*;
use crate::provider::FixtureProvider;
use crate::provider::PanelData;

fn view_mode_defaults() -> GraphStart {
    GraphStart::default()
}

#[test]
fn graph_i18n_keys_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for key in GRAPH_I18N_KEYS {
        assert!(seen.insert(*key), "duplicate key: {key}");
    }
}

#[test]
fn fixture_provider_returns_the_graph() {
    let provider = FixtureProvider::new();
    let data = provider.graph_panel_data().expect("graph");
    assert_eq!(data.nodes.len(), 24);
    assert_eq!(data.components.len(), 2);
    assert_eq!(data.orphans.len(), 3);
}

#[test]
fn default_start_is_full_mode() {
    let start = view_mode_defaults();
    assert!(start.view_mode.is_none());
    assert_eq!(start.ticks, model::DEFAULT_TICKS);
    assert!(!start.settings_open);
}
