//! Shell data provider.
//!
//! [`ShellData`] is the seam between the gpui shell and the headless service
//! layer. [`AppProvider`] drives the real [`StratumService`] on a Tokio
//! runtime; [`FixtureProvider`] returns a deterministic snapshot for tests and
//! for `STRATUM_FIXTURE=1`, which lets the capture harness render the shell
//! without a vault.
//!
//! Recent-page derivation mirrors `src/stores/recentsStore.ts`: journal pages
//! are excluded and the list is ordered by `modified_at` descending. The
//! ordering is a plain string comparison because the backend already emits
//! RFC 3339 timestamps.
//!
//! The fixture generator lives in [`fixture`] so this module stays under the
//! repository file-length limit.

use std::path::PathBuf;
use std::sync::Arc;

use stratum_app::ai::AiAction;
use stratum_app::StratumService;
use stratum_editor::menus::{AiCall, AiCallKind, AiTransformAction};
use stratum_editor::{AssetThumbnail, AttachmentRow, BlockDto};

mod assets;
mod fixture;

pub use assets::attachment_row;
pub(crate) use assets::{
    attachment_row_for_bytes, attachment_row_with_mime, file_extension, file_stem,
    mime_for_extension,
};

pub use fixture::{fixture_blocks, FixtureProvider};

/// Vault counters shown in the sidebar header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultStats {
    /// Number of blocks in the vault.
    pub block_count: u64,
    /// Number of pages in the vault.
    pub page_count: u64,
}

/// A recent page row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentPage {
    /// Vault-relative page path.
    pub path: String,
    /// Display title (falls back to the slug in the provider).
    pub title: String,
    /// Number of blocks on the page.
    pub block_count: u64,
    /// RFC 3339 modification timestamp.
    pub modified_at: String,
}

/// The data the shell needs to render one frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellSnapshot {
    /// Vault counters, absent when the vault cannot be read.
    pub vault: Option<VaultStats>,
    /// Journal-filtered, newest-first recent pages.
    pub recents: Vec<RecentPage>,
}

/// Shell data operations.
pub trait ShellData: Send + Sync {
    /// Read the current snapshot.
    fn snapshot(&self) -> ShellSnapshot;
    /// Refresh and return a new snapshot.
    fn refresh(&self) -> ShellSnapshot;
    /// Export the vault as static HTML, returning the number of pages written.
    fn export_html(&self) -> Result<u64, String>;
    /// Load the blocks of one page. `None` means the page has no blocks (or the
    /// provider cannot read it), and the editor starts empty.
    fn load_blocks(&self, _path: &str) -> Option<Vec<BlockDto>> {
        None
    }
    /// Persist every block of one page. Providers that cannot persist pages
    /// accept the write without error.
    fn save_blocks(&self, _path: &str, _blocks: Vec<BlockDto>) -> Result<(), String> {
        Ok(())
    }
    /// Create a page at `path` and return its stored path.
    ///
    /// The provider appends `.md` when the path has no extension, mirroring
    /// `StratumService::create_page`. Providers that cannot create pages return
    /// the path unchanged.
    fn create_page(&self, _path: &str, _title: Option<&str>) -> Result<String, String> {
        Ok(_path.to_string())
    }
    /// Speak `text` aloud. Providers without audio support return `Ok(())`.
    fn speak(&self, _text: String) -> Result<(), String> {
        Ok(())
    }
    /// Run one AI slash request. Providers without AI support echo the input.
    fn ai_run(&self, call: stratum_editor::menus::AiCall) -> Result<String, String> {
        Ok(call.text)
    }
    /// Read a decoded thumbnail for an image asset.
    ///
    /// Providers without asset support return `None`, and the view keeps the
    /// shipped chip fallback instead of an image preview.
    fn read_asset_thumbnail(&self, _ref_path: &str) -> Option<AssetThumbnail> {
        None
    }
    /// The attachment rows referenced by one page.
    ///
    /// Providers without asset support return an empty list; the panel then
    /// shows its empty state.
    fn page_attachments(&self, _page_path: &str) -> Vec<AttachmentRow> {
        Vec::new()
    }
    /// Import a vault-relative file path as an attachment.
    ///
    /// Providers that cannot persist assets return a row for `path` unchanged.
    fn import_asset_path(&self, path: &str) -> Result<AttachmentRow, String> {
        Ok(attachment_row(path))
    }
    /// Import raw bytes (a clipboard paste or drag-and-drop payload).
    ///
    /// Providers that cannot persist assets return a row under `assets/` with
    /// the byte length and the provided or inferred MIME.
    fn import_asset_bytes(
        &self,
        bytes: Vec<u8>,
        name: &str,
        mime: Option<&str>,
    ) -> Result<AttachmentRow, String> {
        Ok(attachment_row_for_bytes(name, bytes.len() as u64, mime))
    }
    /// Delete an attachment by vault-relative ref path.
    ///
    /// Providers without asset support accept the delete without error.
    fn delete_asset(&self, _ref_path: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Real provider backed by the headless service layer.
pub struct AppProvider {
    service: StratumService,
    runtime: tokio::runtime::Runtime,
}

impl AppProvider {
    /// Create a provider for `vault_path`.
    ///
    /// The runtime is multi-threaded because the service uses
    /// `spawn_blocking` for SQLite and filesystem work.
    pub fn new(vault_path: PathBuf) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build the shell Tokio runtime");
        Self {
            service: StratumService::new(vault_path),
            runtime,
        }
    }

    /// Read vault counters, mapping an unreadable vault to `None`.
    fn read_vault(&self) -> Option<VaultStats> {
        self.runtime
            .block_on(self.service.get_vault_info())
            .ok()
            .map(|info| VaultStats {
                block_count: info.block_count as u64,
                page_count: info.page_count as u64,
            })
    }

    /// Read recent pages, mapping an unreadable page list to an empty list.
    fn read_recents(&self) -> Vec<RecentPage> {
        match self.runtime.block_on(self.service.list_pages()) {
            Ok(list) => derive_recents(
                list.pages
                    .into_iter()
                    .map(|page| RecentPage {
                        path: page.path,
                        title: page.title.unwrap_or(page.slug),
                        block_count: page.block_count as u64,
                        modified_at: page.modified_at,
                    })
                    .collect(),
            ),
            Err(_) => Vec::new(),
        }
    }

    /// Load the editor blocks of one page through the service layer.
    fn read_blocks(&self, path: &str) -> Option<Vec<BlockDto>> {
        self.runtime
            .block_on(self.service.get_blocks(path.to_string()))
            .ok()
            .map(|list| {
                list.blocks
                    .into_iter()
                    .map(app_block_to_editor)
                    .collect::<Vec<_>>()
            })
    }

    /// Persist editor blocks through the service layer.
    fn write_blocks(&self, path: &str, blocks: Vec<BlockDto>) -> Result<(), String> {
        let app_blocks = blocks.into_iter().map(editor_block_to_app).collect();
        self.runtime
            .block_on(self.service.save_blocks(path.to_string(), app_blocks, None))
            .map_err(|error| error.to_string())
    }

    /// Synthesize `text` and start playback on a detached thread.
    ///
    /// Synthesis is fallible and reported to the caller; playback failures are
    /// logged only, because audio is a best-effort side effect and must not turn
    /// an otherwise successful read-aloud request into an error. The thread owns
    /// the `OutputStream` for the whole playback, since dropping it stops audio.
    fn speak_text(&self, text: String) -> Result<(), String> {
        let result = self.runtime.block_on(self.service.tts_speak(text))?;
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(result.audio_b64.as_bytes())
            .map_err(|error| format!("TTS audio decode failed: {error}"))?;
        std::thread::spawn(move || {
            if let Err(error) = play_audio(bytes) {
                log::warn!("read-aloud playback failed: {error}");
            }
        });
        Ok(())
    }
}

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

/// Convert an `stratum-app` block DTO into the editor DTO (field-identical).
fn app_block_to_editor(dto: stratum_app::block::BlockDto) -> BlockDto {
    BlockDto {
        id: dto.id,
        content: dto.content,
        parent_id: dto.parent_id,
        left_id: dto.left_id,
        properties: dto.properties,
        marker: dto.marker,
        priority: dto.priority,
        collapsed: dto.collapsed,
        heading_level: dto.heading_level,
    }
}

/// Convert an editor DTO into the `stratum-app` block DTO (field-identical).
fn editor_block_to_app(dto: BlockDto) -> stratum_app::block::BlockDto {
    stratum_app::block::BlockDto {
        id: dto.id,
        content: dto.content,
        parent_id: dto.parent_id,
        left_id: dto.left_id,
        properties: dto.properties,
        marker: dto.marker,
        priority: dto.priority,
        collapsed: dto.collapsed,
        heading_level: dto.heading_level,
    }
}

/// Decode and play one synthesized audio buffer.
///
/// The `OutputStream` is kept alive on the stack for the duration of playback:
/// dropping it stops the audio device, so it must outlive `sleep_until_end`.
fn play_audio(bytes: Vec<u8>) -> Result<(), String> {
    use std::io::Cursor;

    use rodio::{Decoder, OutputStream, Sink};

    let (_stream, handle) = OutputStream::try_default().map_err(|error| error.to_string())?;
    let sink = Sink::try_new(&handle).map_err(|error| error.to_string())?;
    let source = Decoder::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

/// Derive the recent-page list: exclude the journal namespace and sort by
/// `modified_at` descending. The input is consumed; the returned vector keeps
/// the original block counts.
pub fn derive_recents(pages: Vec<RecentPage>) -> Vec<RecentPage> {
    let mut recents: Vec<RecentPage> = pages
        .into_iter()
        .filter(|page| !page.path.starts_with("journals/"))
        .collect();
    recents.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    recents
}

/// Convenience constructor for the provider selected by the environment.
pub fn provider_from_env(vault_path: PathBuf, fixture: bool) -> Arc<dyn ShellData> {
    if fixture {
        Arc::new(FixtureProvider::new())
    } else {
        Arc::new(AppProvider::new(vault_path))
    }
}

#[cfg(test)]
mod tests;
