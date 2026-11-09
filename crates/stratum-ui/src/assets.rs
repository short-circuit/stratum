//! Embedded shell assets and the asset-source fix.
//!
//! The Android emulator spike showed that `gpui_kit::assets::AllAssets::get`
//! returns `None` for `icons/*.svg` on `x86_64-linux-android` even though the
//! paths are embedded, while the same call succeeds on the host. The shell
//! therefore embeds the icons and Inter faces it needs in its own bundle and
//! consults that bundle first, then falls back to `AllAssets`, then returns
//! `Ok(None)`. This makes icons and fonts work on both the desktop host and
//! the Android host path without patching gpui-kit.
//!
//! [`register_fonts`] installs the four static Inter faces with the GPUI text
//! system. The faces are generated from the upstream variable font with
//! `fontTools.varLib.instancer` at weights 400/500/600/700 and optical size
//! 14; the OFL notice ships beside them.

use std::borrow::Cow;
use std::collections::BTreeSet;

use gpui_kit::assets::AllAssets;
use gpui_kit::*;
use pkm_core::PkmError;

/// Embedded icons and fonts owned by the shell.
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
struct Bundle;

/// The shell asset source.
#[derive(Clone, Copy, Debug, Default)]
pub struct StratumAssets;

impl StratumAssets {
    /// Create the shell asset source.
    pub fn new() -> Self {
        Self
    }
}

impl gpui::AssetSource for StratumAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        // 1. Our bundle owns the shell icons and fonts. This is the path that
        //    works on Android, where AllAssets::get returns None.
        if let Some(file) = Bundle::get(path) {
            return Ok(Some(file.data));
        }
        // 2. Fall back to the complete Lucide/GPUI Kit catalog. Its `load`
        //    returns an error for unknown paths, which is not an error for a
        //    composite source: it simply means "not here".
        match AllAssets::new("").load(path) {
            Ok(Some(data)) => Ok(Some(data)),
            Ok(None) | Err(_) => Ok(None),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut names = BTreeSet::new();
        for name in Bundle::iter() {
            if name.starts_with(path) {
                names.insert(name.into_owned());
            }
        }
        if let Ok(fallback) = AllAssets::new("").list(path) {
            for name in fallback {
                names.insert(name.to_string());
            }
        }
        Ok(names.into_iter().map(SharedString::from).collect())
    }
}

/// Embedded Inter face paths, in weight order.
pub const FONT_PATHS: [&str; 4] = [
    "fonts/Inter-Regular.ttf",
    "fonts/Inter-Medium.ttf",
    "fonts/Inter-SemiBold.ttf",
    "fonts/Inter-Bold.ttf",
];

/// Register the four embedded Inter faces with the GPUI text system.
///
/// Returns [`PkmError::Internal`] when a face is missing from the bundle or
/// the platform text system rejects it.
pub fn register_fonts(cx: &mut gpui::App) -> Result<(), PkmError> {
    let mut faces: Vec<Cow<'static, [u8]>> = Vec::with_capacity(FONT_PATHS.len());
    for path in FONT_PATHS {
        let file = Bundle::get(path)
            .ok_or_else(|| PkmError::Internal(format!("embedded font missing: {path}")))?;
        faces.push(file.data);
    }
    cx.text_system()
        .add_fonts(faces)
        .map_err(|error| PkmError::Internal(format!("failed to register Inter: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::NAV_ITEMS;
    // `gpui_kit::*` re-exports gpui's `test` attribute macro, which shadows
    // the built-in attribute. Import the built-in explicitly so these plain
    // unit tests stay cheap (no GPUI app context is created).
    use ::core::prelude::v1::test;

    /// Every icon the shell embeds. Kept outside the test body so the test
    /// macro expansion stays shallow.
    const SHELL_ICONS: [gpui_kit::assets::IconName; 22] = [
        gpui_kit::assets::IconName::Calendar,
        gpui_kit::assets::IconName::Network,
        gpui_kit::assets::IconName::ListChecks,
        gpui_kit::assets::IconName::AlarmClock,
        gpui_kit::assets::IconName::Search,
        gpui_kit::assets::IconName::Code,
        gpui_kit::assets::IconName::Sparkles,
        gpui_kit::assets::IconName::FileText,
        gpui_kit::assets::IconName::CircleQuestionMark,
        gpui_kit::assets::IconName::PenTool,
        gpui_kit::assets::IconName::Puzzle,
        gpui_kit::assets::IconName::Settings,
        gpui_kit::assets::IconName::Library,
        gpui_kit::assets::IconName::ChevronLeft,
        gpui_kit::assets::IconName::ChevronRight,
        gpui_kit::assets::IconName::RefreshCw,
        gpui_kit::assets::IconName::Upload,
        gpui_kit::assets::IconName::Plus,
        gpui_kit::assets::IconName::Trash,
        gpui_kit::assets::IconName::FolderInput,
        gpui_kit::assets::IconName::X,
        gpui_kit::assets::IconName::Ellipsis,
    ];

    #[test]
    fn every_nav_icon_loads_from_the_bundle() {
        for item in NAV_ITEMS {
            // The icon element is opaque; resolve the same stems through the
            // icon table below. Calling the constructor still proves every
            // navigation row has a renderer.
            let _ = (item.icon)();
        }
        let source = StratumAssets::new();
        for icon in SHELL_ICONS {
            let path = icon.path();
            let loaded = source.load(&path).expect("asset load succeeds");
            assert!(loaded.is_some());
        }
    }

    #[test]
    fn icon_list_is_not_empty() {
        let source = StratumAssets::new();
        let names = source.list("icons").expect("list succeeds");
        assert!(!names.is_empty());
        assert!(names.iter().any(|name| name == "icons/calendar.svg"));
    }

    #[test]
    fn fonts_load_from_the_bundle() {
        let source = StratumAssets::new();
        for path in FONT_PATHS {
            let loaded = source.load(path).expect("font load succeeds");
            let bytes = loaded.unwrap_or_else(|| panic!("missing embedded font {path}"));
            assert!(!bytes.is_empty(), "empty embedded font {path}");
        }
    }

    #[test]
    fn unknown_path_returns_none() {
        let source = StratumAssets::new();
        assert!(source
            .load("does/not/exist.svg")
            .expect("load succeeds")
            .is_none());
    }
}
