//! Graph settings mutators.
//!
//! Extracted from [`super`] so the panel entity module stays inside the
//! repository file-length limit. Every mutator that touches a persisted field
//! saves the row through [`crate::provider::PanelData::save_graph_settings`];
//! the node cap is client-side only and never persisted, matching the shipped
//! panel.

use gpui_kit::prelude::*;

use super::GraphView;

impl GraphView {
    /// Persist the graph settings through the provider seam.
    ///
    /// Mirrors the shipped panel's debounce-save: persistence errors are ignored
    /// so a failed write never blocks the local interaction.
    fn persist_settings(&self) {
        let _ = self.provider.save_graph_settings(self.settings.to_row());
    }

    /// Update a visibility toggle.
    pub(super) fn set_show_connected(&mut self, value: bool, cx: &mut Context<Self>) {
        self.settings.show_connected = value;
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Update a visibility toggle.
    pub(super) fn set_show_orphaned(&mut self, value: bool, cx: &mut Context<Self>) {
        self.settings.show_orphaned = value;
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Update a visibility toggle.
    pub(super) fn set_show_tags(&mut self, value: bool, cx: &mut Context<Self>) {
        self.settings.show_tags = value;
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Switch between the perspective and orthographic camera.
    pub(super) fn set_use_3d(&mut self, value: bool, cx: &mut Context<Self>) {
        self.settings.use_3d = value;
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Update the charge strength.
    pub(super) fn set_charge(&mut self, value: f32, cx: &mut Context<Self>) {
        self.settings.charge_strength = f64::from(value);
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Update the link distance.
    pub(super) fn set_link_distance(&mut self, value: f32, cx: &mut Context<Self>) {
        self.settings.link_distance = f64::from(value);
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Update the alpha decay.
    pub(super) fn set_alpha_decay(&mut self, value: f32, cx: &mut Context<Self>) {
        self.settings.alpha_decay = f64::from(value);
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Update the velocity decay.
    pub(super) fn set_friction(&mut self, value: f32, cx: &mut Context<Self>) {
        self.settings.velocity_decay = f64::from(value);
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Update the link curvature.
    pub(super) fn set_curvature(&mut self, value: f32, cx: &mut Context<Self>) {
        self.settings.link_curvature = f64::from(value);
        self.persist_settings();
        self.rebuild(cx);
    }

    /// Update the explicit node cap.
    ///
    /// The cap is client-side only in the shipped app, so it is never persisted.
    pub(super) fn set_node_cap(&mut self, value: usize, cx: &mut Context<Self>) {
        self.settings.node_cap = value;
        self.rebuild(cx);
    }
}
