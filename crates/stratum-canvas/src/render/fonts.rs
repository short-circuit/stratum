//! Bundled fonts used for deterministic text rasterisation.
//!
//! `resvg` is built without the `system-fonts` feature so the host's font
//! database never influences a render. The Inter face shipped with the native
//! shell (`crates/stratum-ui/assets/fonts/Inter-Regular.ttf`, SIL OFL 1.1,
//! notice alongside it) is embedded here and loaded explicitly by the raster
//! backend. Excalidraw's three text families all map to this face for
//! rasterisation; the SVG writer keeps the Excalidraw family name in a
//! `data-excalidraw-font` attribute so the board stays faithful when reopened.

/// Embedded Inter Regular face.
pub const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");

/// Family name used in exported SVG text.
pub const FONT_FAMILY: &str = "Inter";

/// Map an Excalidraw `fontFamily` wire value to the embedded family name.
///
/// Excalidraw's families are `1` (hand-drawn), `2` (normal) and `3` (code).
/// All three rasterise with Inter; the SVG writer records the original name.
pub fn family_name(font_family: u8) -> &'static str {
    match font_family {
        1 => "Excalifont",
        3 => "Cascadia",
        _ => "Helvetica",
    }
}
