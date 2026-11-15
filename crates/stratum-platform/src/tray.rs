//! Tray icon, menu model and close-to-tray decision.
//!
//! The pure half (menu layout, action ids, close-to-tray rule) is compiled on
//! every target so it can be tested headlessly and reused by the Android host.
//! The OS half builds a `tray-icon` icon. On Linux the crate selects the KSNI
//! backend, which serves the StatusNotifierItem over D-Bus from its own worker
//! thread and therefore needs neither GTK nor a GTK main loop.
//!
//! Menu ids are byte-identical to the shipped Tauri tray (`open`,
//! `quick_capture`, `voice_capture`, `sync_now`, `quit`), so a tray action maps
//! to the same behaviour the webview used to trigger.

use pkm_core::PkmResult;

/// The tray icon id the shell registers.
pub const TRAY_ICON_ID: &str = "stratum-main";

/// One tray menu entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayEntry {
    /// A clickable item.
    Item {
        /// The action the item triggers.
        action: TrayAction,
        /// The localized label shown in the menu.
        label: String,
    },
    /// A separator between groups of items.
    Separator,
}

/// A tray menu action, mirroring the shipped tray menu ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    /// Show and focus the main window.
    Open,
    /// Open the Quick Capture dialog.
    QuickCapture,
    /// Open the Quick Capture dialog in voice mode.
    VoiceCapture,
    /// Run a git sync.
    SyncNow,
    /// Quit the app.
    Quit,
}

impl TrayAction {
    /// The stable menu id used by the OS layer.
    pub fn id(self) -> &'static str {
        match self {
            TrayAction::Open => "open",
            TrayAction::QuickCapture => "quick_capture",
            TrayAction::VoiceCapture => "voice_capture",
            TrayAction::SyncNow => "sync_now",
            TrayAction::Quit => "quit",
        }
    }

    /// Map an OS menu id back to an action.
    ///
    /// Unknown ids are ignored so a future menu entry cannot break the shell.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "open" => Some(TrayAction::Open),
            "quick_capture" => Some(TrayAction::QuickCapture),
            "voice_capture" => Some(TrayAction::VoiceCapture),
            "sync_now" => Some(TrayAction::SyncNow),
            "quit" => Some(TrayAction::Quit),
            _ => None,
        }
    }
}

/// The localized tray menu labels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrayLabels {
    /// "Open Stratum".
    pub open: String,
    /// "Quick Capture".
    pub quick_capture: String,
    /// "Voice Capture".
    pub voice_capture: String,
    /// "Sync Now".
    pub sync_now: String,
    /// "Quit".
    pub quit: String,
}

/// Build the tray menu model.
///
/// The order and the separator position match the shipped Tauri tray exactly:
/// open, quick capture, voice capture, sync now, separator, quit.
pub fn tray_menu(labels: &TrayLabels) -> Vec<TrayEntry> {
    vec![
        TrayEntry::Item {
            action: TrayAction::Open,
            label: labels.open.clone(),
        },
        TrayEntry::Item {
            action: TrayAction::QuickCapture,
            label: labels.quick_capture.clone(),
        },
        TrayEntry::Item {
            action: TrayAction::VoiceCapture,
            label: labels.voice_capture.clone(),
        },
        TrayEntry::Item {
            action: TrayAction::SyncNow,
            label: labels.sync_now.clone(),
        },
        TrayEntry::Separator,
        TrayEntry::Item {
            action: TrayAction::Quit,
            label: labels.quit.clone(),
        },
    ]
}

/// Whether closing the window keeps the process alive in the tray.
///
/// Mirrors `stratum_app::tray::get_effective_close_to_tray`: close-to-tray only
/// applies when the tray itself is enabled and present.
pub fn close_to_tray(tray_enabled: bool, close_to_tray: bool) -> bool {
    tray_enabled && close_to_tray
}

/// A raw RGBA icon for the tray.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayIconImage {
    /// Straight RGBA bytes, four per pixel.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

/// The bundled tray icon (the 32 px application icon).
pub const TRAY_ICON_PNG: &[u8] = include_bytes!("../assets/stratum-tray.png");

impl TrayIconImage {
    /// Decode an embedded PNG into the RGBA buffer `tray-icon` expects.
    pub fn from_png(bytes: &[u8]) -> PkmResult<Self> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .map_err(|e| pkm_core::PkmError::Internal(format!("tray icon: {e}")))?;
        let rgba = image.to_rgba8();
        Ok(Self {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        })
    }

    /// Validate the buffer shape before handing it to the OS.
    pub fn validate(&self) -> PkmResult<()> {
        let expected = (self.width as usize) * (self.height as usize) * 4;
        if self.rgba.len() != expected {
            return Err(pkm_core::PkmError::Validation(format!(
                "tray icon: {} bytes for {}x{}",
                self.rgba.len(),
                self.width,
                self.height
            )));
        }
        Ok(())
    }
}

/// The live tray icon and its menu.
#[cfg(desktop)]
pub struct TrayHandle {
    icon: tray_icon::TrayIcon,
}

#[cfg(desktop)]
impl TrayHandle {
    /// Create the tray icon with the shipped menu.
    ///
    /// The icon is created on the calling thread. On Windows and macOS the
    /// backend pumps events through the application's own event loop, which the
    /// gpui host runs on the main thread.
    pub fn new(labels: &TrayLabels, image: &TrayIconImage, tooltip: &str) -> PkmResult<Self> {
        use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

        image.validate()?;
        let icon = tray_icon::Icon::from_rgba(image.rgba.clone(), image.width, image.height)
            .map_err(|e| pkm_core::PkmError::Internal(format!("tray icon: {e}")))?;

        let mut items: Vec<MenuItem> = Vec::new();
        let menu = Menu::new();
        for entry in tray_menu(labels) {
            match entry {
                TrayEntry::Item { action, label } => {
                    let item = MenuItem::with_id(action.id(), label, true, None);
                    menu.append(&item)
                        .map_err(|e| pkm_core::PkmError::Internal(format!("tray menu: {e}")))?;
                    items.push(item);
                }
                TrayEntry::Separator => {
                    let separator = PredefinedMenuItem::separator();
                    menu.append(&separator)
                        .map_err(|e| pkm_core::PkmError::Internal(format!("tray menu: {e}")))?;
                }
            }
        }

        let icon = tray_icon::TrayIconBuilder::new()
            .with_id(TRAY_ICON_ID)
            .with_menu(Box::new(menu))
            .with_tooltip(tooltip)
            .with_icon(icon)
            .build()
            .map_err(|e| pkm_core::PkmError::Internal(format!("tray icon: {e}")))?;

        Ok(Self { icon })
    }

    /// Show or hide the tray icon.
    pub fn set_visible(&self, visible: bool) -> PkmResult<()> {
        self.icon
            .set_visible(visible)
            .map_err(|e| pkm_core::PkmError::Internal(format!("tray icon: {e}")))
    }

    /// Whether the OS tray exists for this process.
    pub fn is_present(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> TrayLabels {
        TrayLabels {
            open: "Open Stratum".into(),
            quick_capture: "Quick Capture".into(),
            voice_capture: "Voice Capture".into(),
            sync_now: "Sync Now".into(),
            quit: "Quit".into(),
        }
    }

    #[test]
    fn menu_matches_the_shipped_order() {
        let menu = tray_menu(&labels());
        assert_eq!(menu.len(), 6);
        let ids: Vec<&str> = menu
            .iter()
            .filter_map(|entry| match entry {
                TrayEntry::Item { action, .. } => Some(action.id()),
                TrayEntry::Separator => None,
            })
            .collect();
        assert_eq!(
            ids,
            ["open", "quick_capture", "voice_capture", "sync_now", "quit"]
        );
        assert_eq!(menu[4], TrayEntry::Separator);
        assert_eq!(
            menu[0],
            TrayEntry::Item {
                action: TrayAction::Open,
                label: "Open Stratum".into()
            }
        );
    }

    #[test]
    fn action_ids_round_trip() {
        for action in [
            TrayAction::Open,
            TrayAction::QuickCapture,
            TrayAction::VoiceCapture,
            TrayAction::SyncNow,
            TrayAction::Quit,
        ] {
            assert_eq!(TrayAction::from_id(action.id()), Some(action));
        }
        assert_eq!(TrayAction::from_id("nope"), None);
    }

    #[test]
    fn close_to_tray_requires_the_tray() {
        assert!(close_to_tray(true, true));
        assert!(!close_to_tray(false, true));
        assert!(!close_to_tray(true, false));
    }

    #[test]
    fn bundled_icon_decodes_to_the_expected_shape() {
        let image = TrayIconImage::from_png(TRAY_ICON_PNG).expect("decode bundled tray icon");
        image.validate().expect("valid rgba");
        assert_eq!(image.width, 32);
        assert_eq!(image.height, 32);
        assert_eq!(image.rgba.len(), 32 * 32 * 4);
    }

    #[test]
    fn validate_rejects_a_truncated_buffer() {
        let image = TrayIconImage {
            rgba: vec![0; 15],
            width: 2,
            height: 2,
        };
        assert!(image.validate().is_err());
    }
}
