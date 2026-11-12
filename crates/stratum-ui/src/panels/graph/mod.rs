//! Native graph panel.
//!
//! [`GraphView`] is the shipped desktop `GraphPanel` rebuilt on the native shell:
//! it loads the vault graph through [`crate::provider::PanelData`], applies the
//! shipped filter/cap policy, runs the deterministic [`stratum_graph`]
//! simulation, and paints a cached software-rendered frame. The toolbar,
//! settings panel and canvas live in sibling modules so the entity stays inside
//! the repository file-length limit.
//!
//! The panel owns its title header (the shell hides its own header while the
//! entity exists), drains navigation requests for node clicks, and never runs a
//! per-frame simulation: settings or filter changes rebuild the layout, camera
//! interactions only re-rasterize the cached scene.
//!
//! [`GRAPH_I18N_KEYS`] is the release gate list for every string this surface
//! renders.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_graph::{filter_graph, FocusTween, Renderer, Scene, Vec3, ViewMode};
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::ShellData;

pub mod canvas;
pub mod layout;
pub mod model;
pub mod scene;
pub mod setters;
pub mod settings;
pub mod toolbar;
pub mod view;

/// Node count above which the panel lays the graph out on a worker thread.
const ASYNC_LAYOUT_THRESHOLD: usize = 512;

pub use model::{GraphSettings, GraphStart, GraphStatus};

use canvas::CanvasGeometry;

/// Every translation key the graph surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const GRAPH_I18N_KEYS: &[&str] = &[
    "common.loading",
    "common.refresh",
    "graph._cap",
    "graph._large_vault_capped_to",
    "graph.alpha_decay",
    "graph.component",
    "graph.components",
    "graph.connected_notes",
    "graph.could_not_load_graph_data_check_that_your_vault_has_md_files",
    "graph.create_pages_with_wiki_links_to_build_connections",
    "graph.curvature",
    "graph.enable_3d_view",
    "graph.fallback_notice_webgl",
    "graph.filter_by_title_tag",
    "graph.friction",
    "graph.full",
    "graph.graph_settings",
    "graph.graph_settings_label",
    "graph.groups_total",
    "graph.link_distance",
    "graph.n",
    "graph.no_graph_data_yet",
    "graph.node_cap",
    "graph.notes",
    "graph.orphaned_notes",
    "graph.orphans",
    "graph.refresh",
    "graph.repulsion",
    "graph.showing",
    "graph.tags_on_hover",
    "graph.unlimited_auto_cap_for_large_vaults",
    "mobile_nav.graph",
];

/// The graph panel entity.
pub struct GraphView {
    provider: Arc<dyn ShellData>,
    data: Option<stratum_graph::GraphPanelData>,
    filtered: Option<stratum_graph::FilteredGraph>,
    status: GraphStatus,
    loading: bool,
    view_mode: ViewMode,
    selected_component: usize,
    search: String,
    settings: GraphSettings,
    settings_open: bool,
    ticks: usize,
    search_input: Entity<InputState>,
    _input_subscription: Subscription,
    controls: settings::SettingsControls,
    renderer: Option<Renderer>,
    scene: Option<Scene>,
    camera: stratum_graph::Camera,
    positions: Vec<Vec3>,
    layout_data: Option<stratum_graph::GraphData>,
    pending_layout: Option<scene::LayoutJob>,
    image: Option<Arc<RenderImage>>,
    dirty: bool,
    tween: Option<FocusTween>,
    hover: Option<usize>,
    drag: Option<canvas::DragState>,
    focus: FocusHandle,
    geometry: Rc<Cell<CanvasGeometry>>,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl GraphView {
    /// Build the graph panel and load the provider graph.
    ///
    /// `start` carries the deterministic capture state (`STRATUM_GRAPH_*`).
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: GraphStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = t_global("graph.filter_by_title_tag");
        let initial_search = start.search.clone().unwrap_or_default();
        let search_input = cx.new(move |cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(initial_search)
        });
        let input_subscription = cx.subscribe(&search_input, |view, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = view.search_input.read(cx).value().to_string();
                view.set_search(value, cx);
            }
        });
        let settings = match provider.graph_settings() {
            Ok(Some(row)) => {
                let mut settings = GraphSettings::from_row(row);
                // The shipped desktop panel always renders 3D and ignores the
                // persisted flag (it is only consumed by the mobile variant). A
                // mobile start passes `Some(false)` so the graph opens in 2D;
                // otherwise keep the native 3D default.
                settings.use_3d = start.use_3d.unwrap_or(true);
                settings
            }
            _ => {
                let mut settings = GraphSettings::default();
                if let Some(use_3d) = start.use_3d {
                    settings.use_3d = use_3d;
                }
                settings
            }
        };
        let controls = settings::SettingsControls::new(&settings, cx);
        let view_mode = start.view_mode.unwrap_or(ViewMode::Full);
        let mut view = Self {
            provider,
            data: None,
            filtered: None,
            status: GraphStatus::Idle,
            loading: false,
            view_mode,
            selected_component: start.component,
            search: start.search.unwrap_or_default(),
            settings,
            settings_open: start.settings_open,
            ticks: start.ticks.max(1),
            search_input,
            _input_subscription: input_subscription,
            controls,
            renderer: scene::build_renderer().ok(),
            scene: None,
            camera: stratum_graph::Camera::default(),
            positions: Vec::new(),
            layout_data: None,
            pending_layout: None,
            image: None,
            dirty: false,
            tween: None,
            hover: None,
            drag: None,
            focus: cx.focus_handle(),
            geometry: canvas::geometry_cell(),
            pending_navigation: None,
            pending_toast: None,
        };
        view.load(cx);
        view
    }

    /// The current graph data, if loaded.
    pub fn data(&self) -> Option<&stratum_graph::GraphPanelData> {
        self.data.as_ref()
    }

    /// The current filtered render set, if any.
    pub fn filtered(&self) -> Option<&stratum_graph::FilteredGraph> {
        self.filtered.as_ref()
    }

    /// The current settings.
    pub fn settings(&self) -> &GraphSettings {
        &self.settings
    }

    /// The current view mode.
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// The current load state.
    pub fn status(&self) -> &GraphStatus {
        &self.status
    }

    /// The current camera.
    pub fn camera(&self) -> &stratum_graph::Camera {
        &self.camera
    }

    /// The cached render image, if one has been rasterized.
    pub fn image(&self) -> Option<&Arc<RenderImage>> {
        self.image.as_ref()
    }

    /// Whether the settings panel is open.
    pub fn settings_open(&self) -> bool {
        self.settings_open
    }

    /// Whether the cached frame comes from the software rasterizer.
    ///
    /// The shipped app shows `graph.fallback_notice_webgl` when WebGL is
    /// unavailable; the native panel shows the same notice when
    /// [`stratum_graph::Renderer::auto`] selected the deterministic CPU path.
    pub fn software_renderer(&self) -> bool {
        self.renderer
            .as_ref()
            .is_some_and(|renderer| renderer.backend() == "software")
    }

    /// Force the 2D orthographic camera when the software renderer is active.
    ///
    /// The shipped app disables 3D and falls back to the 2D graph when WebGL is
    /// unavailable, so the software rasterizer mirrors that camera mode.
    pub(super) fn apply_renderer_camera_mode(&mut self) {
        if self.software_renderer() {
            self.camera.set_orthographic(true);
        }
    }

    /// Reload the graph from the provider.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    /// Load the graph through the provider seam.
    fn load(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.status = GraphStatus::Loading;
        match self.provider.graph_panel_data() {
            Ok(data) => {
                if data.is_empty() {
                    self.data = None;
                    self.filtered = None;
                    self.scene = None;
                    self.positions.clear();
                    self.layout_data = None;
                    self.pending_layout = None;
                    self.image = None;
                    self.status = GraphStatus::Empty;
                } else {
                    self.data = Some(scene::to_scene_data(&data));
                    self.status = GraphStatus::Ready;
                    self.rebuild(cx);
                }
            }
            Err(error) => {
                self.data = None;
                self.filtered = None;
                self.scene = None;
                self.positions.clear();
                self.layout_data = None;
                self.pending_layout = None;
                self.image = None;
                self.status = GraphStatus::Failed(error);
            }
        }
        self.loading = false;
        cx.notify();
    }

    /// Rebuild the filtered layout and invalidate the cached frame.
    ///
    /// Small graphs build synchronously so fixture captures stay deterministic.
    /// Graphs above the async threshold start a worker-backed [`scene::LayoutJob`]
    /// and keep the previous image visible until the render path polls the
    /// completed layout.
    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let Some(data) = self.data.as_ref() else {
            self.filtered = None;
            self.scene = None;
            self.positions.clear();
            self.layout_data = None;
            self.pending_layout = None;
            self.image = None;
            return;
        };
        let request =
            self.settings
                .filter_request(self.view_mode, self.selected_component, &self.search);
        let filtered = filter_graph(data, &request);
        self.filtered = Some(filtered.clone());
        self.layout_data = Some(stratum_graph::GraphData::new(
            filtered.nodes.clone(),
            filtered.edges.clone(),
            String::new(),
        ));
        if filtered.nodes.len() > ASYNC_LAYOUT_THRESHOLD {
            if let Ok(job) = scene::start_layout(&filtered, &self.settings, self.ticks) {
                self.pending_layout = Some(job);
                self.tween = None;
                if !matches!(self.status, GraphStatus::Failed(_)) {
                    self.status = GraphStatus::Ready;
                }
                cx.notify();
                return;
            }
        }
        self.pending_layout = None;
        match scene::build_layout(&filtered, &self.settings, self.ticks) {
            Ok(layout) => {
                self.scene = Some(layout.scene);
                self.camera = layout.camera;
                self.apply_renderer_camera_mode();
                self.positions = layout.positions;
                self.image = None;
                self.dirty = true;
                self.tween = None;
                if !matches!(self.status, GraphStatus::Failed(_)) {
                    self.status = GraphStatus::Ready;
                }
            }
            Err(error) => {
                self.scene = None;
                self.positions.clear();
                self.image = None;
                self.filtered = None;
                self.layout_data = None;
                self.status = GraphStatus::Failed(error);
            }
        }
        cx.notify();
    }

    /// Re-rasterize the cached frame when the scene or camera is dirty.
    fn ensure_frame(&mut self) -> bool {
        if !self.dirty {
            return self.image.is_some();
        }
        let (Some(scene), Some(renderer)) = (self.scene.as_ref(), self.renderer.as_mut()) else {
            self.dirty = false;
            return false;
        };
        match scene::render_image(scene, &self.camera, renderer) {
            Ok(image) => {
                self.image = Some(image);
                self.dirty = false;
                true
            }
            Err(error) => {
                self.image = None;
                self.dirty = false;
                self.status = GraphStatus::Failed(error);
                false
            }
        }
    }

    /// Advance the focus tween during render and request the next frame.
    fn advance_tween(&mut self, window: &mut Window) {
        let Some(tween) = self.tween.as_mut() else {
            return;
        };
        let running = tween.apply_at(Instant::now(), &mut self.camera);
        if !running {
            self.tween = None;
        }
        self.dirty = true;
        window.request_animation_frame();
    }

    /// Switch the view mode.
    fn set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        self.view_mode = mode;
        self.rebuild(cx);
    }

    /// Cycle to the next connected component.
    fn cycle_component(&mut self, cx: &mut Context<Self>) {
        let count = self
            .data
            .as_ref()
            .map(|data| data.components.len())
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        self.selected_component = (self.selected_component + 1) % count;
        self.rebuild(cx);
    }

    /// Update the search text and switch to full mode when it is non-empty.
    fn set_search(&mut self, value: String, cx: &mut Context<Self>) {
        self.search = value;
        if !self.search.trim().is_empty() {
            self.view_mode = ViewMode::Full;
        }
        self.rebuild(cx);
    }

    /// Toggle the settings panel.
    fn toggle_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = !self.settings_open;
        cx.notify();
    }
}

impl PanelRequests for GraphView {
    fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    fn pending_toast(&self) -> Option<&str> {
        self.pending_toast.as_deref()
    }

    fn take_pending_toast(&mut self) -> Option<String> {
        self.pending_toast.take()
    }
}

#[cfg(test)]
mod tests;
