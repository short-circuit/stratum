//! AI slash-action execution for the native editor.
//!
//! The engine owns the request model and the result application; this module
//! only schedules the provider call on the background executor and surfaces the
//! running/failed status through the save chip.

use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_editor::menus::AiCall;
use stratum_i18n::t_global;

use crate::panels::editor::EditorView;
use crate::provider::ShellData;

/// The AI request currently in flight, or its failure message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiStatus {
    Running(String),
    Failed(String),
}

impl EditorView {
    /// Start one AI request on the background executor.
    ///
    /// The provider call is synchronous (the real provider blocks on its Tokio
    /// runtime), so it must never run on the UI thread. The result is delivered
    /// back through [`Self::finish_ai`] on the main thread.
    pub(crate) fn start_ai(
        &mut self,
        call: AiCall,
        label_key: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.ai_status = Some(AiStatus::Running(t_global(label_key)));
        self.pending_ai = Some(call.clone());
        cx.notify();
        let provider: Arc<dyn ShellData> = self.provider.clone();
        cx.spawn(
            async move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { provider.ai_run(call) })
                    .await;
                let _ = this.update(cx, |view, cx| view.finish_ai(result, cx));
            },
        )
        .detach();
    }

    /// Apply one AI result and update the status chip.
    ///
    /// A non-empty result is applied through the engine; empty results clear the
    /// status without touching the document, and failures keep the document
    /// unchanged while surfacing the provider message.
    pub(crate) fn finish_ai(&mut self, result: Result<String, String>, cx: &mut Context<Self>) {
        match result {
            Ok(content) if !content.trim().is_empty() => {
                if let Some(call) = self.pending_ai.take() {
                    let _ = self.state.apply_ai_content(&call, &content);
                    self.ai_status = None;
                    self.after_edit(cx);
                } else {
                    self.ai_status = None;
                    cx.notify();
                }
            }
            Ok(_) => {
                self.pending_ai = None;
                self.ai_status = None;
                cx.notify();
            }
            Err(message) => {
                self.pending_ai = None;
                self.ai_status = Some(AiStatus::Failed(message));
                cx.notify();
            }
        }
    }

    /// The AI status chip label, if an AI request is running or failed.
    pub(crate) fn ai_label(&self) -> Option<String> {
        match self.ai_status.as_ref()? {
            AiStatus::Running(label) => Some(format!("{label}…")),
            AiStatus::Failed(message) => Some(message.clone()),
        }
    }
}
