//! Theme adaptation helpers used by the gallery preview thumbnail.
//!
//! These functions mirror `src/components/WhiteboardPanel/editorUtils.ts`
//! exactly: dark strokes (luminance below `0.5`) are inverted for a dark
//! preview, light strokes are inverted for a light preview, and transparent
//! fills stay transparent. The luminance formula is the shipped weighted RGB
//! approximation (`0.299/0.587/0.114`), not the sRGB relative-luminance
//! standard, so colours adapt the same way the React panel renders them.
//!
//! Adaptation is opt-in: [`crate::render::SceneOptions::adapt_theme`] enables it
//! for thumbnails only. SVG/PNG/WebP exports keep the document's own colours.

/// Preview background theme for a board thumbnail or export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewTheme {
    /// Light background (`#ffffff`).
    Light,
    /// Dark background (`#232329`).
    Dark,
}

impl PreviewTheme {
    /// The wire value stored in `previewTheme`.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Parse a stored `previewTheme` value.
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

/// Parse `#RRGGBB` into channels. Three-digit and invalid values yield `None`.
fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.strip_prefix('#').unwrap_or(hex);
    if clean.len() != 6 || !clean.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Weighted luminance of a `#RRGGBB` colour in `0.0..=1.0`.
///
/// Invalid input returns `0.0` so callers treat it as a dark colour.
pub fn luminance(hex: &str) -> f64 {
    match parse_hex(hex) {
        Some((r, g, b)) => {
            (f64::from(r) * 0.299 + f64::from(g) * 0.587 + f64::from(b) * 0.114) / 255.0
        }
        None => 0.0,
    }
}

/// Invert a `#RRGGBB` colour channel by channel.
///
/// Invalid input is returned unchanged.
pub fn invert_hex(hex: &str) -> String {
    match parse_hex(hex) {
        Some((r, g, b)) => format!("#{:02x}{:02x}{:02x}", 255 - r, 255 - g, 255 - b),
        None => hex.to_string(),
    }
}

/// Adapt a colour for the requested preview theme.
///
/// `transparent` and non-hex colours are returned unchanged. Hex colours flip
/// when their luminance is on the wrong side of the theme, matching
/// `adaptElementsTheme`.
pub fn adapt_color_for_theme(color: &str, theme: PreviewTheme) -> String {
    if color == "transparent" || !color.starts_with('#') {
        return color.to_string();
    }
    let is_dark = luminance(color) < 0.5;
    let flip = match theme {
        PreviewTheme::Dark => is_dark,
        PreviewTheme::Light => !is_dark,
    };
    if flip {
        invert_hex(color)
    } else {
        color.to_string()
    }
}

/// Background colour for a preview theme.
pub fn theme_background(theme: PreviewTheme) -> String {
    match theme {
        PreviewTheme::Light => "#ffffff".to_string(),
        PreviewTheme::Dark => "#232329".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_dark_stroke_for_dark_theme() {
        assert_eq!(
            adapt_color_for_theme("#1e1e1e", PreviewTheme::Dark),
            invert_hex("#1e1e1e")
        );
        assert_eq!(
            adapt_color_for_theme("#ffffff", PreviewTheme::Dark),
            "#ffffff"
        );
        assert_eq!(
            adapt_color_for_theme("transparent", PreviewTheme::Dark),
            "transparent"
        );
    }

    #[test]
    fn luminance_matches_shipped_formula() {
        assert!(luminance("#000000") < 0.01);
        assert!((luminance("#ffffff") - 1.0).abs() < 1e-9);
    }
}
