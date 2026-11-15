//! Background layout polling and hover highlighting for the graph panel.
//!
//! Large graphs settle on a [`crate::panels::graph::scene::LayoutJob`] worker so
//! the UI thread never blocks. The render path polls the job once per frame and
//! swaps in the completed scene; hover changes rebuild the cached scene with the
//! shipped `SceneOptions.highlight` instead of re-running the simulation.

use gpui_kit::prelude::*;

use super::scene;
use super::{GraphStatus, GraphView};

impl GraphView {
    /// Swap in a completed background layout, if one is ready.
    pub(super) fn poll_layout(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.pending_layout.as_ref() else {
            return;
        };
        let Some(polled) = job.poll() else {
            return;
        };
        self.pending_layout = None;
        let result = match polled {
            Ok(result) => result,
            Err(error) => {
                // The snapshot is consumed; report the failure once instead of
                // requesting animation frames for a job that can never finish.
                self.status = GraphStatus::Failed(error);
                cx.notify();
                return;
            }
        };
        self.scene = Some(result.scene);
        self.camera = result.camera;
        self.apply_renderer_camera_mode();
        self.positions = result.positions;
        self.image = None;
        self.dirty = true;
        self.tween = None;
        if !matches!(self.status, GraphStatus::Failed(_)) {
            self.status = GraphStatus::Ready;
        }
        cx.notify();
    }

    /// Rebuild the scene when the hovered node changes.
    ///
    /// The highlight is a render-only change: it reuses the cached positions and
    /// does not re-run the force simulation.
    pub(super) fn set_hover(&mut self, hover: Option<usize>, cx: &mut Context<Self>) {
        if self.hover == hover {
            return;
        }
        self.hover = hover;
        if self.positions.is_empty() {
            cx.notify();
            return;
        }
        let Some(data) = self.layout_data.as_ref() else {
            cx.notify();
            return;
        };
        let rebuilt = scene::rebuild_scene(data, &self.positions, &self.settings, hover);
        if let Ok(scene) = rebuilt {
            self.scene = Some(scene);
            self.dirty = true;
        }
        cx.notify();
    }
}
