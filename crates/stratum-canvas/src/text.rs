//! Deterministic text metrics, wrapping and line layout.
//!
//! Excalidraw renders text with the bundled Inter faces, but the headless
//! model only needs a stable estimate for hit testing, container sizing and
//! the 500-element text board gate. The advance table below is an
//! approximation of Inter's proportional widths (narrow glyphs ~0.28 em,
//! wide glyphs ~0.95 em); it is intentionally font-data free so results are
//! identical on every platform.
//!
//! Wrapping is O(n): each character is consumed at most once while greedy
//! whitespace wrapping and hard breaks for overlong words are produced.

/// Advance widths for printable ASCII (space through `~`), in em units.
const ASCII_ADVANCE: [f64; 95] = [
    0.26, 0.28, 0.38, 0.60, 0.58, 0.88, 0.68, 0.20, 0.32, 0.32, 0.42, 0.60, 0.28, 0.38, 0.28, 0.34,
    0.58, 0.58, 0.58, 0.58, 0.58, 0.58, 0.58, 0.58, 0.58, 0.58, 0.28, 0.28, 0.60, 0.60, 0.60, 0.48,
    0.90, 0.66, 0.62, 0.66, 0.60, 0.56, 0.56, 0.70, 0.68, 0.28, 0.40, 0.60, 0.52, 0.82, 0.68, 0.72,
    0.60, 0.72, 0.62, 0.58, 0.52, 0.68, 0.64, 0.92, 0.60, 0.60, 0.56, 0.34, 0.34, 0.34, 0.52, 0.50,
    0.28, 0.54, 0.58, 0.52, 0.58, 0.54, 0.34, 0.58, 0.58, 0.26, 0.28, 0.52, 0.26, 0.86, 0.58, 0.56,
    0.58, 0.58, 0.38, 0.50, 0.34, 0.58, 0.54, 0.78, 0.52, 0.54, 0.48, 0.36, 0.26, 0.36, 0.60,
];

/// Fallback advance for characters outside the ASCII table (CJK, emoji,
/// accented glyphs), in em units.
const FALLBACK_ADVANCE: f64 = 0.62;

/// Layout result for a text block.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    /// Wrapped lines.
    pub lines: Vec<String>,
    /// Width of the widest line in world units.
    pub width: f64,
    /// Total height in world units.
    pub height: f64,
}

/// Single-line width of `text` at `font_size`, in world units.
///
/// This is an approximation of Inter's proportional metrics.
pub fn measure(text: &str, font_size: f64) -> f64 {
    text.chars().map(|ch| advance(ch) * font_size).sum::<f64>()
}

/// Advance width of one character in em units.
fn advance(ch: char) -> f64 {
    let code = ch as u32;
    if (0x20..=0x7e).contains(&code) {
        ASCII_ADVANCE[(code - 0x20) as usize]
    } else {
        FALLBACK_ADVANCE
    }
}

/// Height of one text line in world units.
pub fn line_height(font_size: f64, line_height: f64) -> f64 {
    font_size * line_height
}

/// Wrap `text` to `max_width` world units.
///
/// Explicit `\n` breaks are preserved. Words are split on whitespace; a word
/// wider than `max_width` is hard-broken character by character. A
/// non-positive `max_width` disables wrapping.
pub fn wrap(text: &str, font_size: f64, max_width: f64) -> Vec<String> {
    let mut lines = Vec::new();
    for logical in text.split('\n') {
        if max_width <= 0.0 {
            lines.push(logical.to_string());
            continue;
        }
        wrap_line(logical, font_size, max_width, &mut lines);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Wrap one logical line into `out`.
fn wrap_line(line: &str, font_size: f64, max_width: f64, out: &mut Vec<String>) {
    let mut current = String::new();
    let mut current_width = 0.0;
    for word in line.split_whitespace() {
        let word_width = measure(word, font_size);
        let space_width = if current.is_empty() {
            0.0
        } else {
            measure(" ", font_size)
        };
        if word_width > max_width {
            // Flush what we have, then hard-break the overlong word.
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
                current_width = 0.0;
            }
            hard_break(
                word,
                font_size,
                max_width,
                out,
                &mut current,
                &mut current_width,
            );
            continue;
        }
        if !current.is_empty() && current_width + space_width + word_width > max_width {
            out.push(std::mem::take(&mut current));
            current_width = 0.0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_width += space_width;
        }
        current.push_str(word);
        current_width += word_width;
    }
    out.push(current);
}

/// Hard-break an overlong word, leaving any trailing chunk in `current`.
fn hard_break(
    word: &str,
    font_size: f64,
    max_width: f64,
    out: &mut Vec<String>,
    current: &mut String,
    current_width: &mut f64,
) {
    for ch in word.chars() {
        let width = advance(ch) * font_size;
        if !current.is_empty() && *current_width + width > max_width {
            out.push(std::mem::take(current));
            *current_width = 0.0;
        }
        current.push(ch);
        *current_width += width;
    }
}

/// Full layout for a text block: wrapped lines, widest width and total height.
pub fn layout(text: &str, font_size: f64, max_width: f64, line_height: f64) -> TextLayout {
    let lines = wrap(text, font_size, max_width);
    let width = lines
        .iter()
        .map(|line| measure(line, font_size))
        .fold(0.0, f64::max);
    let height = lines.len() as f64 * self::line_height(font_size, line_height);
    TextLayout {
        lines,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_preserves_explicit_newlines() {
        let lines = wrap("a\nb", 20.0, 1000.0);
        assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn overlong_words_are_hard_broken() {
        let lines = wrap("abcdefghij", 20.0, 40.0);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(measure(line, 20.0) <= 40.0 + 1e-6);
        }
        assert_eq!(lines.concat(), "abcdefghij");
    }

    #[test]
    fn layout_height_scales_with_lines() {
        let layout = layout("one two three four", 20.0, 60.0, 1.25);
        assert!(layout.lines.len() > 1);
        assert!((layout.height - layout.lines.len() as f64 * 25.0).abs() < 1e-6);
    }
}
