//! Editor runtime: frame rebuilds, the 800 ms autosave scheduler and the
//! provider-backed flush path.

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::{canvas, model, CanvasView, EditorSession};

impl CanvasView {
    /// Rebuild the cached viewport frame from the open board.
    pub(crate) fn rebuild_frame(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.session.as_mut() else {
            self.frame = None;
            self.frame_image = None;
            return;
        };
        match session.viewport_frame(model::FRAME_WIDTH, model::FRAME_HEIGHT) {
            Ok(frame) => {
                self.frame_image = canvas::render_image(&frame);
                self.frame = Some(frame);
                self.error = None;
            }
            Err(error) => {
                self.frame = None;
                self.frame_image = None;
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.failed_to_generate_thumbnail"),
                    error
                ));
            }
        }
        cx.notify();
    }

    /// Run a canvas edit closure and schedule autosave when it changed the
    /// document.
    pub(crate) fn apply_edit(
        &mut self,
        edit: impl FnOnce(&mut EditorSession, Instant) -> stratum_canvas::CanvasAction,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let action = edit(session, Instant::now());
        if action == stratum_canvas::CanvasAction::DocumentChanged {
            self.rebuild_frame(cx);
            self.schedule_save(cx);
        } else if action != stratum_canvas::CanvasAction::None {
            self.rebuild_frame(cx);
        }
        cx.notify();
    }

    /// Schedule the 800 ms autosave. A generation counter keeps only the newest
    /// timer alive, so a burst of edits writes once.
    pub(crate) fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.save_generation = self.save_generation.wrapping_add(1);
        let generation = self.save_generation;
        cx.spawn(
            async move |this: WeakEntity<CanvasView>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(800))
                    .await;
                let _ = this.update(cx, |view, cx| {
                    if view.save_generation == generation {
                        view.autosave(cx);
                    }
                });
            },
        )
        .detach();
    }

    /// Write pending edits through the provider.
    fn autosave(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let provider = Arc::clone(&self.provider);
        match session.autosave(provider.as_ref(), Instant::now()) {
            Ok(true) => {
                self.message = Some(t_global("whiteboard.saved"));
                self.error = None;
            }
            Ok(false) => {}
            Err(error) => {
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.failed_to_save_whiteboard"),
                    error
                ));
            }
        }
        cx.notify();
    }
}
