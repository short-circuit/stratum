//! Structural SVG path data used by the canvas renderer.
//!
//! `roughr` (the rough.js port used for the Excalidraw-style sketch) emits
//! path strings such as `L0 0 L10 0 C...`. The native gpui panel needs the
//! geometry as numbers so it can feed `lyon`/`PathBuilder`, while the SVG
//! writer needs to re-emit it deterministically. [`PathData`] is that shared
//! representation: [`PathData::parse`] accepts the roughr/rough.js grammar,
//! [`PathData::to_d`] serialises it with fixed precision and
//! [`PathData::transformed`] applies the export/viewport affine.
//!
//! Note that roughr writes the first `Move` operation as `L` (SVG treats a
//! leading line-to as a move-to), so a leading `LineTo` is normalised to
//! `MoveTo` here. `Z` is accepted and re-emitted even though roughr never
//! emits it. Malformed input is rejected with [`pkm_core::PkmError::Canvas`].

use pkm_core::{PkmError, PkmResult};

use crate::geometry::{Affine, Point};

/// One structural path operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathOp {
    /// Start a new sub-path at the given point.
    MoveTo(Point),
    /// Straight segment to the given point.
    LineTo(Point),
    /// Cubic Bézier segment.
    CurveTo {
        /// First control point.
        c1: Point,
        /// Second control point.
        c2: Point,
        /// End point.
        to: Point,
    },
    /// Close the current sub-path.
    Close,
}

/// Parsed path data.
#[derive(Debug, Clone, PartialEq)]
pub struct PathData {
    /// Operations in order.
    pub ops: Vec<PathOp>,
}

impl PathData {
    /// Parse roughr/rough.js path data (`M`, `L`, `C`, `Z`, comma/space
    /// separated). Lower-case commands are accepted as relative operations.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] for unknown commands, missing operands,
    /// non-finite numbers or a path that starts with a curve.
    pub fn parse(d: &str) -> PkmResult<Self> {
        let mut parser = Parser::new(d);
        let mut ops = Vec::new();
        let mut cursor = Point::new(0.0, 0.0);
        let mut started = false;
        while let Some(command) = parser.next_command() {
            match command {
                'M' | 'm' => {
                    let point = parser.point(command == 'm', cursor)?;
                    ops.push(PathOp::MoveTo(point));
                    cursor = point;
                    started = true;
                }
                'L' | 'l' => {
                    let point = parser.point(command == 'l', cursor)?;
                    if !started {
                        // roughr encodes the opening move as `L`; SVG renders
                        // a leading line-to as a move-to, so mirror that.
                        ops.push(PathOp::MoveTo(point));
                        started = true;
                    } else {
                        ops.push(PathOp::LineTo(point));
                    }
                    cursor = point;
                }
                'C' | 'c' => {
                    if !started {
                        return Err(PkmError::Canvas(
                            "path starts with a curve segment".to_string(),
                        ));
                    }
                    let c1 = parser.point(command == 'c', cursor)?;
                    let c2 = parser.point(command == 'c', cursor)?;
                    let to = parser.point(command == 'c', cursor)?;
                    ops.push(PathOp::CurveTo { c1, c2, to });
                    cursor = to;
                }
                'Z' | 'z' => {
                    ops.push(PathOp::Close);
                }
                other => {
                    return Err(PkmError::Canvas(format!(
                        "unsupported path command '{other}'"
                    )))
                }
            }
        }
        if ops.is_empty() {
            return Err(PkmError::Canvas("empty path data".to_string()));
        }
        Ok(Self { ops })
    }

    /// Serialise deterministically as `M x y L x y C ... Z` with three
    /// decimals.
    pub fn to_d(&self) -> String {
        let mut out = String::new();
        for op in &self.ops {
            match op {
                PathOp::MoveTo(p) => {
                    out.push_str("M ");
                    push_point(&mut out, *p);
                }
                PathOp::LineTo(p) => {
                    out.push_str(" L ");
                    push_point(&mut out, *p);
                }
                PathOp::CurveTo { c1, c2, to } => {
                    out.push_str(" C ");
                    push_point(&mut out, *c1);
                    out.push(' ');
                    push_point(&mut out, *c2);
                    out.push(' ');
                    push_point(&mut out, *to);
                }
                PathOp::Close => out.push_str(" Z"),
            }
        }
        out
    }

    /// Apply an affine transform to every point.
    pub fn transformed(&self, m: &Affine) -> PathData {
        let ops = self
            .ops
            .iter()
            .map(|op| match op {
                PathOp::MoveTo(p) => PathOp::MoveTo(m.apply(*p)),
                PathOp::LineTo(p) => PathOp::LineTo(m.apply(*p)),
                PathOp::CurveTo { c1, c2, to } => PathOp::CurveTo {
                    c1: m.apply(*c1),
                    c2: m.apply(*c2),
                    to: m.apply(*to),
                },
                PathOp::Close => PathOp::Close,
            })
            .collect();
        PathData { ops }
    }
}

/// Format one coordinate with three decimals and no negative zero.
pub(crate) fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "0.000".to_string();
    }
    let rounded = if value.abs() < 0.0005 { 0.0 } else { value };
    format!("{rounded:.3}")
}

/// Append a point in `x y` form.
fn push_point(out: &mut String, p: Point) {
    out.push_str(&format_number(p.x));
    out.push(' ');
    out.push_str(&format_number(p.y));
}

/// Tokenizer for the path grammar.
struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    /// Build a parser over `d`.
    fn new(d: &'a str) -> Self {
        Self {
            bytes: d.as_bytes(),
            index: 0,
        }
    }

    /// Skip separators and return the next command letter.
    fn next_command(&mut self) -> Option<char> {
        self.skip_separators();
        let byte = *self.bytes.get(self.index)?;
        if byte.is_ascii_alphabetic() {
            self.index += 1;
            Some(char::from(byte))
        } else {
            // A number without a preceding command is invalid; return the
            // byte as a pseudo-command so the caller reports it.
            Some(char::from(byte))
        }
    }

    /// Skip whitespace and commas.
    fn skip_separators(&mut self) {
        while let Some(byte) = self.bytes.get(self.index) {
            if byte.is_ascii_whitespace() || *byte == b',' {
                self.index += 1;
            } else {
                break;
            }
        }
    }

    /// Parse one point, applying relative offsets when requested.
    fn point(&mut self, relative: bool, cursor: Point) -> PkmResult<Point> {
        let x = self.number()?;
        let y = self.number()?;
        if relative {
            Ok(Point::new(cursor.x + x, cursor.y + y))
        } else {
            Ok(Point::new(x, y))
        }
    }

    /// Parse one number.
    fn number(&mut self) -> PkmResult<f64> {
        self.skip_separators();
        let start = self.index;
        if matches!(self.bytes.get(self.index), Some(b'+') | Some(b'-')) {
            self.index += 1;
        }
        let mut digits = 0;
        while let Some(byte) = self.bytes.get(self.index) {
            if byte.is_ascii_digit() {
                digits += 1;
                self.index += 1;
            } else {
                break;
            }
        }
        if matches!(self.bytes.get(self.index), Some(b'.')) {
            self.index += 1;
            while let Some(byte) = self.bytes.get(self.index) {
                if byte.is_ascii_digit() {
                    digits += 1;
                    self.index += 1;
                } else {
                    break;
                }
            }
        }
        if digits == 0 {
            return Err(PkmError::Canvas("missing path operand".to_string()));
        }
        if matches!(self.bytes.get(self.index), Some(b'e') | Some(b'E')) {
            self.index += 1;
            if matches!(self.bytes.get(self.index), Some(b'+') | Some(b'-')) {
                self.index += 1;
            }
            let mut exponent_digits = 0;
            while let Some(byte) = self.bytes.get(self.index) {
                if byte.is_ascii_digit() {
                    exponent_digits += 1;
                    self.index += 1;
                } else {
                    break;
                }
            }
            if exponent_digits == 0 {
                return Err(PkmError::Canvas("malformed path exponent".to_string()));
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.index])
            .map_err(|e| PkmError::Canvas(e.to_string()))?;
        let value: f64 = text
            .parse()
            .map_err(|_| PkmError::Canvas(format!("invalid path number '{text}'")))?;
        if !value.is_finite() {
            return Err(PkmError::Canvas(format!("non-finite path number '{text}'")));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roughr::core::OptionsBuilder;
    use roughr::generator::Generator;

    /// Generate the exact `d` strings roughr emits for a rectangle and an
    /// ellipse so the parser is tested against real output.
    fn rough_paths() -> Vec<String> {
        let generator = Generator::default();
        let options = OptionsBuilder::default()
            .seed(7_u64)
            .stroke_width(2.0_f32)
            .build()
            .expect("options");
        let mut paths = Vec::new();
        for drawable in [
            generator.rectangle(0.0_f64, 0.0_f64, 40.0_f64, 20.0_f64, &Some(options.clone())),
            generator.ellipse(20.0_f64, 10.0_f64, 40.0_f64, 20.0_f64, &Some(options)),
        ] {
            paths.extend(Generator::to_paths(drawable).into_iter().map(|p| p.d));
        }
        paths
    }

    #[test]
    fn parses_real_roughr_output() {
        for d in rough_paths() {
            let data = PathData::parse(&d).unwrap_or_else(|e| panic!("{d}: {e}"));
            assert!(matches!(data.ops[0], PathOp::MoveTo(_)));
            assert!(data.ops.len() > 1);
        }
    }

    #[test]
    fn round_trips_deterministically() {
        let data = PathData::parse("L0 0 L10 0 L10 10 Z").unwrap();
        let first = data.to_d();
        let second = PathData::parse(&first).unwrap().to_d();
        assert_eq!(first, second);
        assert_eq!(first, "M 0.000 0.000 L 10.000 0.000 L 10.000 10.000 Z");
    }

    #[test]
    fn relative_commands_are_applied() {
        let data = PathData::parse("m 10 10 l 5 0 c 1 1 2 2 3 3").unwrap();
        assert_eq!(data.ops[0], PathOp::MoveTo(Point::new(10.0, 10.0)));
        assert_eq!(data.ops[1], PathOp::LineTo(Point::new(15.0, 10.0)));
        match data.ops[2] {
            PathOp::CurveTo { to, .. } => assert_eq!(to, Point::new(18.0, 13.0)),
            _ => panic!("expected curve"),
        }
    }

    #[test]
    fn transform_applies_to_every_point() {
        let data = PathData::parse("M 0 0 L 10 0").unwrap();
        let m = Affine {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 2.0,
            e: 5.0,
            f: 7.0,
        };
        let moved = data.transformed(&m);
        assert_eq!(moved.ops[0], PathOp::MoveTo(Point::new(5.0, 7.0)));
        assert_eq!(moved.ops[1], PathOp::LineTo(Point::new(25.0, 7.0)));
    }

    #[test]
    fn malformed_paths_are_canvas_errors() {
        assert!(matches!(PathData::parse("M 0"), Err(PkmError::Canvas(_))));
        assert!(matches!(
            PathData::parse("C 0 0 1 1 2 2"),
            Err(PkmError::Canvas(_))
        ));
        assert!(matches!(PathData::parse("X 1 2"), Err(PkmError::Canvas(_))));
        assert!(matches!(PathData::parse(""), Err(PkmError::Canvas(_))));
        assert!(matches!(
            PathData::parse("M nan 0"),
            Err(PkmError::Canvas(_))
        ));
    }
}
