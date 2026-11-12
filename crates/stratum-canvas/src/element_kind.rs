//! Element kinds, style enums and the string mappings used by the
//! `.excalidraw` format.
//!
//! The shipped whiteboard stores Excalidraw JSON verbatim, so every enum here
//! has an exact wire representation (for example `fillStyle: "cross-hatch"`).
//! Unknown values are preserved through [`ElementKind::Unknown`] and the
//! `From<&str>` helpers instead of being rejected, which keeps round-trips
//! lossless when a newer Excalidraw adds a kind or a style value.

use std::fmt;

/// The Excalidraw element discriminator (`type`).
///
/// Unknown values are kept as [`ElementKind::Unknown`] so a board written by a
/// newer Excalidraw version still round-trips without dropping the element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementKind {
    /// `rectangle`.
    Rectangle,
    /// `ellipse`.
    Ellipse,
    /// `diamond`.
    Diamond,
    /// `line` (polyline without arrowheads).
    Line,
    /// `arrow` (polyline with arrowheads).
    Arrow,
    /// `freedraw` (pencil stroke).
    Freedraw,
    /// `text`.
    Text,
    /// `image`.
    Image,
    /// `frame`.
    Frame,
    /// `embeddable`.
    Embeddable,
    /// `iframe`.
    Iframe,
    /// `magicframe`.
    MagicFrame,
    /// Any other `type` value, preserved verbatim.
    Unknown(String),
}

impl ElementKind {
    /// Parse the wire `type` value.
    ///
    /// Unknown strings become [`ElementKind::Unknown`] rather than an error so
    /// that forward-compatible boards stay editable.
    pub fn from_wire(value: &str) -> Self {
        match value {
            "rectangle" => Self::Rectangle,
            "ellipse" => Self::Ellipse,
            "diamond" => Self::Diamond,
            "line" => Self::Line,
            "arrow" => Self::Arrow,
            "freedraw" => Self::Freedraw,
            "text" => Self::Text,
            "image" => Self::Image,
            "frame" => Self::Frame,
            "embeddable" => Self::Embeddable,
            "iframe" => Self::Iframe,
            "magicframe" => Self::MagicFrame,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// The wire `type` value written into the element JSON.
    pub fn as_wire(&self) -> &str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Diamond => "diamond",
            Self::Line => "line",
            Self::Arrow => "arrow",
            Self::Freedraw => "freedraw",
            Self::Text => "text",
            Self::Image => "image",
            Self::Frame => "frame",
            Self::Embeddable => "embeddable",
            Self::Iframe => "iframe",
            Self::MagicFrame => "magicframe",
            Self::Unknown(value) => value,
        }
    }

    /// Whether the element is a point-based polyline (`line`, `arrow`,
    /// `freedraw`). Those elements carry a `points` array and are hit-tested
    /// against their stroke instead of their bounding box.
    pub fn is_linear(&self) -> bool {
        matches!(self, Self::Line | Self::Arrow | Self::Freedraw)
    }

    /// Whether the element can contain bound text (`rectangle`, `ellipse`,
    /// `diamond`, `text`). Used to bind a text element to its container.
    pub fn is_bindable_container(&self) -> bool {
        matches!(
            self,
            Self::Rectangle | Self::Ellipse | Self::Diamond | Self::Text
        )
    }

    /// Whether the element is a closed shape (rectangle/ellipse/diamond).
    /// Shape tools use this to decide whether drag-to-create applies.
    pub fn is_shape(&self) -> bool {
        matches!(self, Self::Rectangle | Self::Ellipse | Self::Diamond)
    }
}

impl fmt::Display for ElementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_wire())
    }
}

/// `fillStyle` values supported by Excalidraw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillStyle {
    /// `hachure` (default hand-drawn fill).
    Hachure,
    /// `cross-hatch`.
    CrossHatch,
    /// `solid`.
    Solid,
    /// `zigzag`.
    ZigZag,
    /// `dots`.
    Dots,
}

impl FillStyle {
    /// Parse the wire value, defaulting to [`FillStyle::Hachure`].
    pub fn from_wire(value: &str) -> Self {
        match value {
            "cross-hatch" => Self::CrossHatch,
            "solid" => Self::Solid,
            "zigzag" => Self::ZigZag,
            "dots" => Self::Dots,
            _ => Self::Hachure,
        }
    }

    /// The exact Excalidraw wire string.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Hachure => "hachure",
            Self::CrossHatch => "cross-hatch",
            Self::Solid => "solid",
            Self::ZigZag => "zigzag",
            Self::Dots => "dots",
        }
    }
}

/// `strokeStyle` values supported by Excalidraw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeStyle {
    /// `solid` (default).
    Solid,
    /// `dashed`.
    Dashed,
    /// `dotted`.
    Dotted,
}

impl StrokeStyle {
    /// Parse the wire value, defaulting to [`StrokeStyle::Solid`].
    pub fn from_wire(value: &str) -> Self {
        match value {
            "dashed" => Self::Dashed,
            "dotted" => Self::Dotted,
            _ => Self::Solid,
        }
    }

    /// The exact Excalidraw wire string.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Dashed => "dashed",
            Self::Dotted => "dotted",
        }
    }
}

/// Corner sharpness. The legacy `strokeSharpness` string is still read when a
/// board predates the canonical `roundness` object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeSharpness {
    /// `sharp` — square corners.
    Sharp,
    /// `round` — rounded corners.
    Round,
}

impl StrokeSharpness {
    /// Parse the legacy `strokeSharpness` value, defaulting to sharp.
    pub fn from_wire(value: &str) -> Self {
        match value {
            "round" => Self::Round,
            _ => Self::Sharp,
        }
    }

    /// The legacy wire string (only written for compatibility with older
    /// consumers; canonical boards use `roundness`).
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Sharp => "sharp",
            Self::Round => "round",
        }
    }
}

/// Text font families exposed by Excalidraw (`fontFamily`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    /// Hand-drawn font (`1`).
    HandDrawn,
    /// Normal font (`2`).
    Normal,
    /// Code font (`3`).
    Code,
}

impl FontFamily {
    /// Parse the numeric `fontFamily` value, defaulting to [`FontFamily::Normal`].
    pub fn from_wire(value: u64) -> Self {
        match value {
            1 => Self::HandDrawn,
            3 => Self::Code,
            _ => Self::Normal,
        }
    }

    /// The numeric wire value.
    pub fn as_wire(&self) -> u64 {
        match self {
            Self::HandDrawn => 1,
            Self::Normal => 2,
            Self::Code => 3,
        }
    }
}

/// Arrowhead styles (`startArrowhead`/`endArrowhead`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrowhead {
    /// `arrow` (open V).
    Arrow,
    /// `triangle`.
    Triangle,
    /// `circle`.
    Circle,
    /// `diamond`.
    Diamond,
    /// `bar`.
    Bar,
}

impl Arrowhead {
    /// Parse the wire value; returns `None` for `null`/unknown values.
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "arrow" => Some(Self::Arrow),
            "triangle" => Some(Self::Triangle),
            "circle" => Some(Self::Circle),
            "diamond" => Some(Self::Diamond),
            "bar" => Some(Self::Bar),
            _ => None,
        }
    }

    /// The exact Excalidraw wire string.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Arrow => "arrow",
            Self::Triangle => "triangle",
            Self::Circle => "circle",
            Self::Diamond => "diamond",
            Self::Bar => "bar",
        }
    }
}

/// Horizontal text alignment (`textAlign`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// `left`.
    Left,
    /// `center`.
    Center,
    /// `right`.
    Right,
}

impl TextAlign {
    /// Parse the wire value, defaulting to [`TextAlign::Left`].
    pub fn from_wire(value: &str) -> Self {
        match value {
            "center" => Self::Center,
            "right" => Self::Right,
            _ => Self::Left,
        }
    }

    /// The exact Excalidraw wire string.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }
}

/// Vertical text alignment (`verticalAlign`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    /// `top`.
    Top,
    /// `middle`.
    Middle,
    /// `bottom`.
    Bottom,
}

impl VerticalAlign {
    /// Parse the wire value, defaulting to [`VerticalAlign::Top`].
    pub fn from_wire(value: &str) -> Self {
        match value {
            "middle" => Self::Middle,
            "bottom" => Self::Bottom,
            _ => Self::Top,
        }
    }

    /// The exact Excalidraw wire string.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Middle => "middle",
            Self::Bottom => "bottom",
        }
    }
}
