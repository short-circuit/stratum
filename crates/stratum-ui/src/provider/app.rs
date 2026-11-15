//! `ShellData` implementation for the service-backed provider.
//!
//! Kept in its own module so `provider/mod.rs` stays under the repository
//! file-length limit. Page-specific mapping lives in [`super::page`], journal
//! mapping in [`super::journal`], voice in [`super::voice`], assets in
//! [`super::assets`] and read-aloud in [`super::audio`].

use stratum_app::ai::AiAction;
use stratum_editor::menus::{AiCall, AiCallKind, AiTransformAction};
use stratum_editor::{AssetThumbnail, AttachmentRow, BlockDto};

use super::{
    AppProvider, BacklinkRow, BacklinkSnippet, ConnectionRow, PageInfo, ReindexSummary, ShellData,
    ShellSnapshot,
};

impl ShellData for AppProvider {
    fn snapshot(&self) -> ShellSnapshot {
        ShellSnapshot {
            vault: self.read_vault(),
            recents: self.read_recents(),
        }
    }

    fn refresh(&self) -> ShellSnapshot {
        self.snapshot()
    }

    fn export_html(&self) -> Result<u64, String> {
        self.runtime
            .block_on(self.service.export_html("/tmp/stratum-export".to_string()))
            .map(|result| result.pages_exported as u64)
    }

    fn load_blocks(&self, path: &str) -> Option<Vec<BlockDto>> {
        self.read_blocks(path)
    }

    fn save_blocks(&self, path: &str, blocks: Vec<BlockDto>) -> Result<(), String> {
        self.write_blocks(path, blocks)
    }

    fn create_page(&self, path: &str, title: Option<&str>) -> Result<String, String> {
        self.runtime
            .block_on(
                self.service
                    .create_page(path.to_string(), title.map(str::to_string)),
            )
            .map(|page| page.path)
    }

    fn quick_capture_save(&self, text: String) -> Result<super::QuickCaptureResult, String> {
        let saved = self
            .runtime
            .block_on(self.service.quick_capture_save(text))?;
        Ok(super::QuickCaptureResult {
            page_path: saved.page_path,
            block_id: saved.block_id,
        })
    }

    fn speak(&self, text: String) -> Result<(), String> {
        self.speak_text(text)
    }

    fn read_asset_thumbnail(&self, ref_path: &str) -> Option<AssetThumbnail> {
        self.read_thumbnail(ref_path)
    }

    fn page_attachments(&self, page_path: &str) -> Vec<AttachmentRow> {
        self.read_attachments(page_path)
    }

    fn import_asset_path(&self, path: &str) -> Result<AttachmentRow, String> {
        self.import_path(path)
    }

    fn import_asset_bytes(
        &self,
        bytes: Vec<u8>,
        name: &str,
        mime: Option<&str>,
    ) -> Result<AttachmentRow, String> {
        self.import_bytes(bytes, name, mime)
    }

    fn delete_asset(&self, ref_path: &str) -> Result<(), String> {
        self.delete_ref(ref_path)
    }

    fn journal_dates(&self) -> Vec<String> {
        self.read_journal_dates()
    }

    fn ensure_today_journal(&self) -> Result<String, String> {
        self.ensure_journal()
    }

    fn create_journal_day(&self, date: &str) -> Result<String, String> {
        self.create_journal(date)
    }

    fn repair_db(&self) -> Result<(), String> {
        self.repair_database()
    }

    fn stt_configured(&self) -> bool {
        self.stt_is_configured()
    }

    fn voice_start(&self, page_path: &str) -> Result<String, String> {
        self.begin_recording(page_path)
    }

    fn voice_stop(&self) -> Result<(String, f64), String> {
        self.finish_recording()
    }

    fn voice_transcribe(&self, recording_path: &str, page_path: &str) -> Result<usize, String> {
        self.transcribe_clip(recording_path, page_path)
    }

    fn voice_insert_clip(
        &self,
        recording_path: &str,
        page_path: &str,
        seconds: f64,
    ) -> Result<(), String> {
        self.insert_clip(recording_path, page_path, seconds)
    }

    fn page_info(&self, path: &str) -> Option<PageInfo> {
        self.read_page_info(path)
    }

    fn reindex_page(&self, path: &str) -> Result<ReindexSummary, String> {
        self.reindex_one_page(path)
    }

    fn normalize_page(&self, path: &str) -> Result<(), String> {
        self.normalize_one_page(path)
    }

    fn move_page(&self, old_path: &str, new_path: &str) -> Result<String, String> {
        self.move_one_page(old_path, new_path)
    }

    fn delete_page(&self, path: &str) -> Result<(), String> {
        self.delete_one_page(path)
    }

    fn backlinks(&self, path: &str) -> Vec<BacklinkRow> {
        self.read_backlinks(path)
    }

    fn backlink_snippet(&self, note_id: &str, anchor_id: &str) -> Option<BacklinkSnippet> {
        self.read_backlink_snippet(note_id, anchor_id)
    }

    fn suggest_connections(&self, path: &str) -> Result<Vec<ConnectionRow>, String> {
        self.read_suggestions(path)
    }

    fn insert_block(&self, path: &str, markdown: &str) -> Result<(), String> {
        self.append_block(path, markdown)
    }

    fn ai_run(&self, call: AiCall) -> Result<String, String> {
        match call.kind {
            AiCallKind::Transform(action) => {
                let action = match action {
                    AiTransformAction::Rewrite => AiAction::Rewrite,
                    AiTransformAction::Format => AiAction::Format,
                    AiTransformAction::Structure => AiAction::Structure,
                    AiTransformAction::Summarize => AiAction::Summarize,
                    AiTransformAction::Connect => AiAction::Connect,
                };
                self.runtime
                    .block_on(
                        self.service
                            .ai_transform_block(call.text, action, call.page_path),
                    )
                    .map(|result| result.content)
            }
            AiCallKind::Research => self
                .runtime
                .block_on(self.service.ai_research(call.text))
                .map(|result| result.findings),
            AiCallKind::Interlink => self
                .runtime
                .block_on(self.service.ai_interlink_notes(call.text, call.page_path))
                .map(|result| result.content),
            AiCallKind::Mermaid => self
                .runtime
                .block_on(self.service.generate_mermaid(call.text))
                .map(|result| result.content),
        }
    }
}
