//! Safe-area and IME geometry for the mobile shell.
//!
//! The model is deliberately pure so the layout contract is testable without a
//! window. [`frame_from_insets`] converts the platform's [`WindowInsets`] into
//! logical-pixel paddings clamped to the viewport, and [`frame_for_window`]
//! reads the live insets from a gpui [`Window`].
//!
//! ## Capture affordance
//!
//! Headless captures cannot observe a real notch, home indicator or keyboard.
//! When `STRATUM_INSETS` is set to `top,right,bottom,left,ime` (logical pixels),
//! [`frame_for_window`] parses it and overrides the platform values. Malformed
//! input is ignored so the platform value stays authoritative. This exists only
//! for the capture harness; interactive builds never need it.

use gpui_kit::*;

/// The safe-area and keyboard geometry the mobile shell lays out against.
///
/// Every field is in logical pixels and clamped to the viewport by
/// [`frame_from_insets`]. `ime` is the keyboard height exposed separately so the
/// Ask composer can raise above it while the rest of the content keeps the
/// safe-area padding.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MobileFrame {
    /// Top safe-area inset (status bar, notch).
    pub top: f32,
    /// Right safe-area inset.
    pub right: f32,
    /// Bottom safe-area inset (home indicator, gesture bar).
    pub bottom: f32,
    /// Left safe-area inset.
    pub left: f32,
    /// Keyboard (IME) height.
    pub ime: f32,
}

impl MobileFrame {
    /// Whether every edge is zero (desktop or an inset-free viewport).
    pub fn is_zero(&self) -> bool {
        self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0 && self.left == 0.0
    }
}

/// Build the mobile frame from platform insets.
///
/// Safe-area edges come from [`WindowInsets::safe_area`] and are clamped to the
/// viewport so an oversized platform value can never push content off-screen.
/// The keyboard height is exposed separately as [`MobileFrame::ime`] rather
/// than folded into [`MobileFrame::bottom`]: the bottom navigation bar stays
/// above the home indicator while the Ask composer can raise above the keyboard
/// without double-padding the rest of the content.
pub fn frame_from_insets(insets: &WindowInsets, viewport: Size<Pixels>) -> MobileFrame {
    let safe = &insets.safe_area;
    let width = viewport.width.as_f32().max(0.0);
    let height = viewport.height.as_f32().max(0.0);
    MobileFrame {
        top: clamp_edge(safe.top.as_f32(), height),
        right: clamp_edge(safe.right.as_f32(), width),
        bottom: clamp_edge(safe.bottom.as_f32(), height),
        left: clamp_edge(safe.left.as_f32(), width),
        ime: clamp_edge(insets.ime.bottom.as_f32(), height),
    }
}

/// Parse the `STRATUM_INSETS` capture override.
///
/// Accepts exactly five comma-separated non-negative finite numbers in the
/// order `top,right,bottom,left,ime`. Returns `None` for any malformed input so
/// the caller keeps the platform value.
pub fn parse_insets(value: &str) -> Option<MobileFrame> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    if parts.len() != 5 {
        return None;
    }
    let mut values = [0.0_f32; 5];
    for (slot, part) in values.iter_mut().zip(parts) {
        let parsed: f32 = part.parse().ok()?;
        if !parsed.is_finite() || parsed < 0.0 {
            return None;
        }
        *slot = parsed;
    }
    Some(MobileFrame {
        top: values[0],
        right: values[1],
        bottom: values[2],
        left: values[3],
        ime: values[4],
    })
}

/// Apply an optional capture override to a platform-derived frame.
pub fn with_env_override(frame: MobileFrame, value: Option<&str>) -> MobileFrame {
    value.and_then(parse_insets).unwrap_or(frame)
}

/// Read the live frame for `window`.
///
/// Uses the platform's [`Window::insets`], then applies the `STRATUM_INSETS`
/// capture override when it parses. A malformed override keeps the platform
/// value.
pub fn frame_for_window(window: &Window) -> MobileFrame {
    let viewport = window.bounds().size;
    let frame = frame_from_insets(&window.insets(), viewport);
    with_env_override(frame, std::env::var("STRATUM_INSETS").ok().as_deref())
}

/// Clamp one edge to `0..=limit`, treating NaN as zero.
fn clamp_edge(value: f32, limit: f32) -> f32 {
    if !value.is_finite() || value < 0.0 {
        0.0
    } else {
        value.min(limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    fn viewport() -> Size<Pixels> {
        size(px(1080.0), px(2400.0))
    }

    #[test]
    fn zero_insets_produce_a_zero_frame() {
        let frame = frame_from_insets(&WindowInsets::default(), viewport());
        assert_eq!(frame, MobileFrame::default());
        assert!(frame.is_zero());
    }

    #[test]
    fn notch_home_indicator_and_ime_are_exact() {
        let mut insets = WindowInsets::default();
        insets.safe_area.top = px(48.0);
        insets.safe_area.bottom = px(72.0);
        insets.ime.bottom = px(320.0);
        let frame = frame_from_insets(&insets, viewport());
        assert_eq!(frame.top, 48.0);
        assert_eq!(frame.bottom, 72.0);
        assert_eq!(frame.ime, 320.0);
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    #[test]
    fn oversized_insets_clamp_to_the_viewport() {
        let mut insets = WindowInsets::default();
        insets.safe_area.top = px(5000.0);
        insets.safe_area.bottom = px(9000.0);
        insets.safe_area.left = px(4000.0);
        insets.safe_area.right = px(4000.0);
        insets.ime.bottom = px(9000.0);
        let frame = frame_from_insets(&insets, viewport());
        assert_eq!(frame.top, 2400.0);
        assert_eq!(frame.bottom, 2400.0);
        assert_eq!(frame.left, 1080.0);
        assert_eq!(frame.right, 1080.0);
        assert_eq!(frame.ime, 2400.0);
    }

    #[test]
    fn negative_insets_are_treated_as_zero() {
        let mut insets = WindowInsets::default();
        insets.safe_area.top = px(-20.0);
        insets.ime.bottom = px(-5.0);
        let frame = frame_from_insets(&insets, viewport());
        assert_eq!(frame.top, 0.0);
        assert_eq!(frame.ime, 0.0);
    }

    #[test]
    fn env_parser_accepts_the_capture_format() {
        let frame = parse_insets("48,0,72,0,0").expect("valid capture insets");
        assert_eq!(
            frame,
            MobileFrame {
                top: 48.0,
                right: 0.0,
                bottom: 72.0,
                left: 0.0,
                ime: 0.0,
            }
        );
        assert_eq!(
            parse_insets(" 48 , 2 , 72 , 4 , 300 "),
            Some(MobileFrame {
                top: 48.0,
                right: 2.0,
                bottom: 72.0,
                left: 4.0,
                ime: 300.0,
            })
        );
    }

    #[test]
    fn env_parser_rejects_malformed_input() {
        assert_eq!(parse_insets(""), None);
        assert_eq!(parse_insets("48,0,72,0"), None);
        assert_eq!(parse_insets("48,0,72,0,0,0"), None);
        assert_eq!(parse_insets("48,0,72,0,abc"), None);
        assert_eq!(parse_insets("48,0,72,0,-1"), None);
        assert_eq!(parse_insets("48,0,72,0,inf"), None);
        assert_eq!(parse_insets("48,0,72,0,NaN"), None);
    }

    #[test]
    fn malformed_env_override_keeps_the_platform_frame() {
        let platform = MobileFrame {
            top: 24.0,
            right: 0.0,
            bottom: 16.0,
            left: 0.0,
            ime: 0.0,
        };
        assert_eq!(with_env_override(platform, Some("nope")), platform);
        assert_eq!(with_env_override(platform, None), platform);
        let overridden = with_env_override(platform, Some("48,0,72,0,0"));
        assert_eq!(overridden.top, 48.0);
        assert_eq!(overridden.bottom, 72.0);
    }
}
