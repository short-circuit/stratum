//! Headless accessibility probe for the native shell.
//!
//! Opens the deterministic fixture shell, renders a fixed number of frames and
//! dumps the AccessKit tree that gpui reports to the platform adapter
//! (`Window::debug_a11y_tree_json`). The probe prints one JSON object:
//!
//! ```json
//! {"a11y": "inactive"}
//! ```
//! when no assistive technology is connected (gpui only builds the tree while
//! accessibility is active), or
//! ```json
//! {"a11y": "active", "nodes": 42, "tree": {...}}
//! ```
//! otherwise. `scripts/capture-gpui-a11y.sh` runs it under an AT-SPI session and
//! `scripts/verify-gpui-a11y.sh` asserts on the tree.
//!
//! Exit codes: 0 on a successful dump (active or inactive), 1 when the window
//! never rendered.

use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_ui::provider::provider_from_env;
use stratum_ui::{init, Shell};

/// How many frames to render before dumping the tree.
///
/// gpui paints on demand, so the probe explicitly requests the second frame;
/// the tree harvested at the end of frame 1 is what frame 2 sees.
const DUMP_FRAME: usize = 2;

/// A probe root that renders the shell and dumps the a11y tree once the UI has
/// settled.
struct ProbeRoot {
    shell: Entity<Shell>,
    frames: usize,
    dumped: bool,
}

impl Render for ProbeRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.frames += 1;
        if self.frames >= DUMP_FRAME && !self.dumped {
            self.dumped = true;
            match window.debug_a11y_tree_json() {
                Some(json) => println!("{}", json),
                None => println!("{{\"a11y\": \"inactive\"}}"),
            }
            cx.quit();
        } else {
            // gpui paints on demand: request the frame that carries the tree
            // harvested from this one.
            cx.notify();
        }
        div().size_full().child(self.shell.clone())
    }
}

fn main() {
    let fixture = std::env::var("STRATUM_FIXTURE")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    let vault = std::env::var("STRATUM_VAULT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        let provider = provider_from_env(vault.clone(), fixture);
        init(cx, &Default::default());
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(0.0), px(0.0)),
                size(px(1440.0), px(900.0)),
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Stratum".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let shell = cx.new(|_| Shell::new(Arc::clone(&provider)));
        let handle = gpui_kit::open_window(options, cx, move |_window, cx| {
            let shell = shell.clone();
            cx.new(move |_| ProbeRoot {
                shell,
                frames: 0,
                dumped: false,
            })
        });
        if handle.is_err() {
            eprintln!("a11y-probe: failed to open the shell window");
            std::process::exit(1);
        }
        cx.activate(true);
    });
}
