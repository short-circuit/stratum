//! Window-free tests for the native Whiteboards surface.
//!
//! These exercise the pure gallery/session model the gpui panel delegates to:
//! gallery create/rename/multi-select/delete, byte-identical untouched saves,
//! the 800 ms autosave with preview refresh, SVG/PNG export, the library
//! round-trip, clipboard copy/paste, style/align/distribute, `Board#Element`
//! links and deep-link focus, the SceneCache visibility query on a 5k board,
//! and the [`super::WHITEBOARD_I18N_KEYS`] catalog gate across all eight
//! locales.

use std::time::{Duration, Instant};

use stratum_canvas::render::perf::stress_board;
use stratum_canvas::{
    Align, Axis, BoardDocument, BoardLink, Element, ElementKind, FillStyle, PreviewTheme, Style,
    StylePatch, Tool,
};
use stratum_i18n::{t, Locale};

use super::model::{EditorSession, GalleryModel, AUTOSAVE_DELAY, FRAME_HEIGHT, FRAME_WIDTH};
use super::WHITEBOARD_I18N_KEYS;
use crate::provider::{FixtureProvider, PanelData};

fn provider() -> FixtureProvider {
    FixtureProvider::new()
}

fn open(provider: &FixtureProvider, name: &str) -> EditorSession {
    GalleryModel::open(provider, name, PreviewTheme::Light).expect("fixture board opens")
}

#[test]
fn gallery_loads_previews_and_supports_crud() {
    let provider = provider();
    let mut gallery = GalleryModel::load(&provider).expect("gallery loads");
    assert_eq!(gallery.rows().len(), 3);
    let architecture = gallery
        .rows()
        .iter()
        .find(|row| row.name == "Architecture sketch")
        .expect("fixture board");
    assert!(architecture.element_count >= 4);
    let blank = gallery
        .rows()
        .iter()
        .find(|row| row.name == "Blank canvas")
        .expect("fixture board");
    assert_eq!(blank.element_count, 0);

    gallery.create(&provider, "Scratch board").expect("create");
    assert_eq!(gallery.rows().len(), 4);
    gallery
        .rename(&provider, "Scratch board", "Renamed board")
        .expect("rename");
    assert!(gallery.rows().iter().any(
        |row| row.name == "Renamed board" && row.path == "whiteboards/Renamed board.excalidraw"
    ));

    gallery.toggle_selection("Flow sketch");
    gallery.toggle_selection("Renamed board");
    assert_eq!(
        gallery.selected_names(),
        vec!["Flow sketch".to_string(), "Renamed board".to_string()]
    );
    assert_eq!(gallery.delete_selected(&provider).expect("delete"), 2);
    assert_eq!(gallery.rows().len(), 2);
    assert!(!gallery.rows().iter().any(|row| row.name == "Flow sketch"));
}

#[test]
fn untouched_boards_serialize_byte_identically_and_do_not_flush() {
    let provider = provider();
    let content = provider
        .load_whiteboard("Architecture sketch")
        .expect("fixture board");
    let mut session = EditorSession::open("Architecture sketch", &content, PreviewTheme::Light)
        .expect("board parses");
    assert_eq!(session.serialized().expect("serialize"), content);
    assert!(!session.flush(&provider, Instant::now()).expect("flush"));
    assert_eq!(
        provider
            .load_whiteboard("Architecture sketch")
            .expect("fixture board"),
        content
    );
}

#[test]
fn autosave_waits_eight_hundred_millis_and_stores_a_preview() {
    let provider = provider();
    let mut session = open(&provider, "Flow sketch");
    session.state_mut().document_mut();
    let start = Instant::now();
    session.note_edit(start);
    assert!(!session.save_due(start + Duration::from_millis(799)));
    assert!(session.save_due(start + AUTOSAVE_DELAY));
    assert!(session
        .autosave(&provider, start + AUTOSAVE_DELAY)
        .expect("autosave"));
    assert!(session.thumbnail().is_some());
    let stored = provider
        .load_whiteboard("Flow sketch")
        .expect("saved board");
    let doc = BoardDocument::load(stored.as_bytes()).expect("saved board parses");
    assert!(doc.preview().is_some());
}

#[test]
fn exports_svg_and_png_at_twice_the_board_size() {
    let provider = provider();
    let mut session = open(&provider, "Architecture sketch");
    let svg_path = session
        .export(&provider, super::ExportFormat::Svg)
        .expect("svg export");
    assert!(svg_path.ends_with("Architecture sketch.svg"));
    let png_path = session
        .export(&provider, super::ExportFormat::Png)
        .expect("png export");
    assert!(png_path.ends_with("Architecture sketch.png"));
    let base = session
        .state_mut()
        .export_png(&Default::default(), None)
        .expect("base png");
    let base = image::load_from_memory_with_format(&base, image::ImageFormat::Png)
        .expect("base png decodes");
    let bytes = session.png_bytes(&Default::default()).expect("png bytes");
    let image =
        image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).expect("png decodes");
    // Rounding can differ by one pixel between the 1x and 2x raster paths.
    let scale = super::model::EXPORT_SCALE as i64;
    assert!((i64::from(image.width()) - i64::from(base.width()) * scale).abs() <= 1);
    assert!((i64::from(image.height()) - i64::from(base.height()) * scale).abs() <= 1);
}

#[test]
fn library_insert_and_save_selection_round_trip() {
    let provider = provider();
    let mut session = open(&provider, "Architecture sketch");
    session
        .state_mut()
        .select_ids(&["fixture-rect".to_string(), "fixture-ellipse".to_string()]);
    let item = session
        .save_selection_to_library(&provider, "Architecture parts")
        .expect("selection saved");
    assert_eq!(item.name(), Some("Architecture parts"));
    let rows = provider.whiteboard_library().expect("library");
    assert!(rows.len() >= 2);
    assert!(rows
        .iter()
        .any(|row| row.name.as_deref() == Some("Architecture parts")));

    let existing = rows
        .iter()
        .find(|row| row.name.as_deref() == Some("Architecture sketch"))
        .expect("fixture library item")
        .to_item()
        .expect("item parses");
    let before = session.state().document().visible_elements().count();
    let inserted = session
        .insert_library_item(&existing, 600.0, 400.0, Instant::now())
        .expect("insert");
    assert!(!inserted.is_empty());
    assert!(
        session.state().document().visible_elements().count() > before,
        "library insert adds elements"
    );
}

#[test]
fn clipboard_copy_and_paste_round_trip() {
    let provider = provider();
    let mut session = open(&provider, "Architecture sketch");
    session
        .state_mut()
        .select_ids(&["fixture-rect".to_string()]);
    let payload = session.copy_selection();
    assert!(payload.contains("fixture-rect"));

    let mut target = EditorSession::empty("Paste target", PreviewTheme::Light).expect("empty");
    let action = target
        .paste(&payload, Instant::now())
        .expect("paste parses");
    assert_eq!(action, stratum_canvas::CanvasAction::DocumentChanged);
    assert_eq!(target.state().document().visible_elements().count(), 1);

    assert!(target.paste("not json", Instant::now()).is_err());
}

#[test]
fn style_align_and_distribute_apply_to_the_selection() {
    let provider = provider();
    let mut session = open(&provider, "Blank canvas");
    for (index, x) in [10.0, 120.0, 300.0].into_iter().enumerate() {
        let mut element = Element::new(
            ElementKind::Rectangle,
            &format!("style-{index}"),
            x,
            20.0,
            60.0,
            40.0,
        );
        element.set_style(&Style {
            background_color: "#a5d8ff".to_string(),
            fill_style: FillStyle::Solid,
            ..Style::default()
        });
        session
            .state_mut()
            .document_mut()
            .add_element(element)
            .expect("element");
    }
    let ids = vec![
        "style-0".to_string(),
        "style-1".to_string(),
        "style-2".to_string(),
    ];
    session.state_mut().select_ids(&ids);
    session.align(Align::Left, Instant::now());
    let xs: Vec<f64> = ids
        .iter()
        .map(|id| session.state().document().element(id).unwrap().x())
        .collect();
    assert!(xs.windows(2).all(|pair| pair[0] == pair[1]));

    session
        .state_mut()
        .document_mut()
        .element_mut("style-0")
        .unwrap()
        .set_extra("x", serde_json::json!(0.0));
    session.distribute(Axis::Horizontal, Instant::now());

    session.set_style(
        &StylePatch {
            stroke_color: Some("#e03131".to_string()),
            ..StylePatch::default()
        },
        Instant::now(),
    );
    for id in &ids {
        assert_eq!(
            session
                .state()
                .document()
                .element(id)
                .unwrap()
                .style()
                .stroke_color,
            "#e03131"
        );
    }
}

#[test]
fn links_and_deep_link_focus_resolve_frames_and_elements() {
    let provider = provider();
    let mut session = open(&provider, "Architecture sketch");
    let link = session.copy_link(Some("fixture-rect"));
    assert_eq!(
        link,
        "/whiteboards?board=Architecture%20sketch&element=fixture-rect"
    );
    let target = crate::router::parse_target(&link).expect("link parses");
    assert_eq!(target.param("board"), Some("Architecture sketch"));
    assert_eq!(target.param("element"), Some("fixture-rect"));

    let resolved = session.focus_element("Architecture").expect("frame focus");
    assert_eq!(resolved.frame_id.as_deref(), Some("fixture-frame"));
    assert_eq!(session.state().selection(), ["fixture-frame".to_string()]);

    let resolved = session
        .focus_element("fixture-rect")
        .expect("element focus");
    assert_eq!(resolved.element_id.as_deref(), Some("fixture-rect"));
    assert_eq!(session.state().selection(), ["fixture-rect".to_string()]);
    assert!(session.focus_element("missing").is_err());
    assert_eq!(
        BoardLink::parse("Architecture sketch#fixture-rect")
            .expect("link")
            .to_string(),
        "Architecture sketch#fixture-rect"
    );
}

#[test]
fn five_thousand_element_board_uses_the_visibility_query() {
    let mut doc = stress_board(5_000, 0);
    let content = doc.save_string().expect("serialize");
    let mut session =
        EditorSession::open("Stress", &content, PreviewTheme::Light).expect("board parses");
    let visible = session
        .visible(FRAME_WIDTH, FRAME_HEIGHT)
        .expect("visible query");
    let count =
        visible.paths.len() + visible.texts.len() + visible.images.len() + visible.frames.len();
    assert!(count > 0, "the viewport shows part of the board");
    assert!(count < 5_000, "the viewport culls off-screen elements");

    session.zoom(0.5, Instant::now());
    let zoomed = session
        .visible(FRAME_WIDTH, FRAME_HEIGHT)
        .expect("visible query");
    let zoomed_count =
        zoomed.paths.len() + zoomed.texts.len() + zoomed.images.len() + zoomed.frames.len();
    assert!(zoomed_count >= count, "zooming out reveals more elements");
}

#[test]
fn whiteboard_keys_are_present_in_every_catalog_and_unique() {
    let mut seen = std::collections::HashSet::new();
    for key in WHITEBOARD_I18N_KEYS {
        assert!(seen.insert(*key), "duplicate key: {key}");
        for locale in Locale::ALL {
            let value = t(locale, key);
            assert_ne!(value, *key, "{key} missing in {}", locale.code());
        }
    }
    for key in [
        "whiteboard.back_to_gallery",
        "whiteboard.copy_link",
        "whiteboard.export_png",
        "whiteboard.export_svg",
        "whiteboard.insert",
        "whiteboard.library",
        "whiteboard.save_selection_to_library",
        "whiteboard.style",
        "whiteboard.tool_label",
        "whiteboard.undo",
    ] {
        assert!(
            WHITEBOARD_I18N_KEYS.contains(&key),
            "{key} is missing from the release gate list"
        );
    }
}

#[test]
fn tool_and_export_start_state_parse_from_env() {
    assert_eq!(super::tool_from_env("frame"), Some(Tool::Frame));
    assert_eq!(super::tool_from_env("nope"), None);
    assert_eq!(
        super::ExportFormat::from_env("png"),
        Some(super::ExportFormat::Png)
    );
    assert_eq!(
        super::ExportFormat::from_env("svg"),
        Some(super::ExportFormat::Svg)
    );
}
