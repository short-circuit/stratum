//! Colour parsing for the roughr renderer.
//!
//! Excalidraw stores colours as `#RRGGBB`/`#RRGGBBAA`, `rgb(...)`/`rgba(...)`
//! or one of a small set of names. `roughr` needs a `palette::Srgba`, so this
//! module converts the wire colour into 8-bit channels and then into the
//! float component type roughr accepts. Invalid colours are not fatal: the
//! caller keeps the original string for SVG output and falls back to black
//! for the sketch geometry.

use palette::Srgba;

/// Fully transparent detection matching Excalidraw's `isTransparent`.
///
/// `transparent`, `#RGB0`/`#RGBA` with a zero alpha nibble and `#RRGGBBAA`
/// with a zero alpha byte are all transparent.
pub fn is_transparent(color: &str) -> bool {
    let lower = color.trim().to_ascii_lowercase();
    if lower == "transparent" {
        return true;
    }
    if let Some(hex) = lower.strip_prefix('#') {
        match hex.len() {
            4 => return hex.as_bytes().get(3) == Some(&b'0'),
            8 => return &hex[6..8] == "00",
            _ => {}
        }
    }
    if lower.starts_with("rgba(") {
        let inner = lower.trim_start_matches("rgba(").trim_end_matches(')');
        let mut parts = inner.split(',');
        let _ = parts.next();
        let _ = parts.next();
        let _ = parts.next();
        if let Some(alpha) = parts.next() {
            let alpha = alpha.trim();
            if alpha.parse::<f64>().map(|v| v <= 0.0).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}

/// Parse a colour into `(r, g, b, a)` bytes.
///
/// Returns `None` when the colour is transparent, empty or unrecognised.
pub fn parse_color(color: &str) -> Option<(u8, u8, u8, u8)> {
    if is_transparent(color) {
        return None;
    }
    let lower = color.trim().to_ascii_lowercase();
    if let Some(hex) = lower.strip_prefix('#') {
        return parse_hex(hex);
    }
    if lower.starts_with("rgb(") || lower.starts_with("rgba(") {
        return parse_rgb(&lower);
    }
    named(&lower).map(|(r, g, b)| (r, g, b, 255))
}

/// Build the roughr colour type from parsed channels, multiplying the alpha
/// by an element opacity percentage.
pub fn to_srgba(rgba: (u8, u8, u8, u8), opacity: f64) -> Srgba {
    let alpha = (f64::from(rgba.3) * (opacity / 100.0).clamp(0.0, 1.0)) as u8;
    Srgba::new(
        f32::from(rgba.0) / 255.0,
        f32::from(rgba.1) / 255.0,
        f32::from(rgba.2) / 255.0,
        f32::from(alpha) / 255.0,
    )
}

/// Parse the hex body (without `#`).
fn parse_hex(hex: &str) -> Option<(u8, u8, u8, u8)> {
    let expand = |c: char| -> Option<u8> { c.to_digit(16).map(|v| (v * 17) as u8) };
    match hex.len() {
        3 => {
            let mut chars = hex.chars();
            let r = expand(chars.next()?)?;
            let g = expand(chars.next()?)?;
            let b = expand(chars.next()?)?;
            Some((r, g, b, 255))
        }
        4 => {
            let mut chars = hex.chars();
            let r = expand(chars.next()?)?;
            let g = expand(chars.next()?)?;
            let b = expand(chars.next()?)?;
            let a = expand(chars.next()?)?;
            Some((r, g, b, a))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b, 255))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some((r, g, b, a))
        }
        _ => None,
    }
}

/// Parse `rgb(...)`/`rgba(...)` with integer channels.
fn parse_rgb(color: &str) -> Option<(u8, u8, u8, u8)> {
    let inner = color
        .strip_prefix("rgba(")
        .or_else(|| color.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let channel = |value: &str| -> Option<u8> {
        if let Some(percent) = value.strip_suffix('%') {
            let value: f64 = percent.parse().ok()?;
            return Some((value.clamp(0.0, 100.0) * 2.55).round() as u8);
        }
        let value: f64 = value.parse().ok()?;
        Some(value.clamp(0.0, 255.0).round() as u8)
    };
    let r = channel(parts[0])?;
    let g = channel(parts[1])?;
    let b = channel(parts[2])?;
    let a = match parts.get(3) {
        Some(raw) => {
            let value: f64 = raw.parse().ok()?;
            if value <= 1.0 {
                (value * 255.0).round() as u8
            } else {
                value.clamp(0.0, 255.0).round() as u8
            }
        }
        None => 255,
    };
    Some((r, g, b, a))
}

/// Named colours accepted by Excalidraw's colour set.
fn named(color: &str) -> Option<(u8, u8, u8)> {
    let rgb = match color {
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "blue" => (0, 0, 255),
        "green" => (0, 128, 0),
        "yellow" => (255, 255, 0),
        "orange" => (255, 165, 0),
        "violet" => (238, 130, 238),
        "gray" | "grey" => (128, 128, 128),
        _ => return None,
    };
    Some(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_alpha() {
        assert_eq!(parse_color("#1e1e1e"), Some((30, 30, 30, 255)));
        assert_eq!(parse_color("#a5d8ffcc"), Some((165, 216, 255, 204)));
        assert_eq!(parse_color("#fff"), Some((255, 255, 255, 255)));
        assert_eq!(parse_color("#1e1e1e00"), None);
        assert_eq!(parse_color("transparent"), None);
    }

    #[test]
    fn parses_rgb_and_named() {
        assert_eq!(parse_color("rgb(10, 20, 30)"), Some((10, 20, 30, 255)));
        assert_eq!(parse_color("rgba(1,2,3,0.5)"), Some((1, 2, 3, 128)));
        assert_eq!(parse_color("red"), Some((255, 0, 0, 255)));
        assert_eq!(parse_color("chartreuse"), None);
    }

    #[test]
    fn opacity_multiplies_alpha() {
        let srgba = to_srgba((10, 20, 30, 255), 50.0);
        assert!((srgba.alpha - 127.0 / 255.0).abs() < 1e-6);
    }
}
