//! Canvas export CLI.
//!
//! Loads an `.excalidraw` board and writes SVG, PNG, lossless WebP or a
//! preview thumbnail. The export path is the same code the native panel uses
//! (`stratum_canvas::render`), so the CLI doubles as a visual check.
//!
//! Usage:
//!
//! ```text
//! canvas_export --input board.excalidraw --output board.svg --format svg
//! canvas_export --input board.excalidraw --output board.png --format png --max-edge 2048
//! canvas_export --input board.excalidraw --output board.webp --format webp --theme dark
//! canvas_export --input board.excalidraw --output thumb.webp --format thumbnail
//! ```
//!
//! `thumbnail` also prints the WebP `data:` URL to stdout. All failures exit
//! non-zero with a message on stderr.

use std::fs;
use std::process::ExitCode;

use base64::Engine;
use stratum_canvas::render::raster::{export_png, export_webp, thumbnail};
use stratum_canvas::render::svg::scene_svg;
use stratum_canvas::render::{Scene, SceneOptions};
use stratum_canvas::{BoardDocument, PreviewTheme};

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Svg,
    Png,
    WebP,
    Thumbnail,
}

/// Parsed command line.
struct Args {
    input: String,
    output: String,
    format: Format,
    theme: PreviewTheme,
    max_edge: Option<u32>,
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("canvas_export: {error}");
            return ExitCode::FAILURE;
        }
    };
    match run(&args) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("canvas_export: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Parse the command line.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut input = None;
    let mut output = None;
    let mut format = Format::Svg;
    let mut theme = PreviewTheme::Light;
    let mut max_edge = None;
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--input" => input = Some(value),
            "--output" => output = Some(value),
            "--format" => format = parse_format(&value)?,
            "--theme" => theme = parse_theme(&value)?,
            "--max-edge" => {
                max_edge = Some(
                    value
                        .parse::<u32>()
                        .map_err(|error| format!("--max-edge: {error}"))?,
                )
            }
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(Args {
        input: input.ok_or_else(|| "--input is required".to_string())?,
        output: output.ok_or_else(|| "--output is required".to_string())?,
        format,
        theme,
        max_edge,
    })
}

/// Parse the output format.
fn parse_format(value: &str) -> Result<Format, String> {
    match value {
        "svg" => Ok(Format::Svg),
        "png" => Ok(Format::Png),
        "webp" => Ok(Format::WebP),
        "thumbnail" => Ok(Format::Thumbnail),
        other => Err(format!("unknown format {other}")),
    }
}

/// Parse the preview theme.
fn parse_theme(value: &str) -> Result<PreviewTheme, String> {
    PreviewTheme::from_wire(value).ok_or_else(|| format!("unknown theme {value}"))
}

/// Run one export.
fn run(args: &Args) -> Result<String, String> {
    let bytes = fs::read(&args.input).map_err(|error| format!("read {}: {error}", args.input))?;
    let doc = BoardDocument::load(&bytes).map_err(|error| error.to_string())?;
    let options = SceneOptions {
        theme: args.theme,
        ..SceneOptions::default()
    };
    let body = match args.format {
        Format::Svg => {
            let scene = Scene::build(&doc, &options).map_err(|error| error.to_string())?;
            scene_svg(&scene).into_bytes()
        }
        Format::Png => {
            export_png(&doc, &options, args.max_edge).map_err(|error| error.to_string())?
        }
        Format::WebP => {
            export_webp(&doc, &options, args.max_edge).map_err(|error| error.to_string())?
        }
        Format::Thumbnail => {
            let thumb = thumbnail(&doc, args.theme).map_err(|error| error.to_string())?;
            println!("{}", thumb.data_url);
            base64::engine::general_purpose::STANDARD
                .decode(
                    thumb
                        .data_url
                        .strip_prefix("data:image/webp;base64,")
                        .ok_or_else(|| "thumbnail is not a webp data URL".to_string())?,
                )
                .map_err(|error| error.to_string())?
        }
    };
    fs::write(&args.output, &body).map_err(|error| format!("write {}: {error}", args.output))?;
    Ok(format!(
        "wrote {} bytes to {} ({:?}, {:?})",
        body.len(),
        args.output,
        args.format,
        args.theme
    ))
}
