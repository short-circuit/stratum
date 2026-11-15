//! Deterministic graph physics, camera parity and a headless 3D renderer for the
//! Stratum graph panel.
//!
//! The crate mirrors the shipped React/three.js behaviour without depending on
//! `stratum-app`, `tauri`, three.js or a windowing system:
//!
//! * [`physics`] ports `d3-force-3d` (Barnes-Hut octree, link, collide and the
//!   shipped z-spring) and exposes a background [`SimWorker`].
//! * [`camera`] reproduces the `TrackballControls` orbit/pan/dolly behaviour,
//!   the shipped WASD step, the 500 ms focus tween, the shipped
//!   `3d-force-graph` initial camera placement and a `fit_to_positions` helper.
//! * [`data`], [`filter`] and [`scene`] mirror the graph DTOs, the shipped
//!   filtering/capping policy and the renderable scene snapshot.
//! * [`render`] builds headless wgpu pipelines (nodes, links, SDF labels,
//!   picking) and a deterministic CPU fallback for hosts without an adapter.
//! * [`perf`] collects frame timings and enforces the golden-run FPS gate.
//!
//! The deterministic fixture generator in [`data::fixture`] and the golden
//! runner in `src/bin/graph_golden.rs` make the crate testable without a vault.

pub mod camera;
pub mod curve;
pub mod data;
pub mod filter;
pub mod math;
pub mod perf;
pub mod physics;
pub mod render;
pub mod scene;
pub mod worker;

pub use camera::{
    Camera, FocusTween, MoveKey, CAMERA_DISTANCE2NODES_FACTOR, FOCUS_DURATION, PAN_SPEED,
    WASD_SPEED, ZOOM_SPEED,
};
pub use curve::{link_curve_control, link_curve_points, link_perpendicular, LINK_CURVE_SEGMENTS};
pub use data::{
    fixture, node_color, seed_position, Component, GraphData, GraphEdge, GraphNode, GraphPanelData,
    Orphan, PALETTE,
};
pub use filter::{
    apply_cap, cap_nodes_by_degree, edges_within, effective_node_cap, filter_graph, FilteredGraph,
    GraphFilterRequest, GraphVisibility, ViewMode, DEFAULT_LARGE_VAULT_CAP, LARGE_VAULT_THRESHOLD,
};
pub use math::{Mat4, Vec3};
pub use perf::{
    peak_rss_mb, FrameBucket, FrameTimer, PerfReport, HISTOGRAM_EDGES_MS, MIN_GOLDEN_FPS,
    WORK_BUDGET_MS,
};
pub use physics::barnes_hut::THETA2;
pub use physics::init::{spherical_position, Lcg};
pub use physics::sim::{ForceParams, SimNode, Simulation};
pub use render::{device::GpuContext, sdf::GlyphAtlas, Frame, RenderConfig, Renderer};
pub use scene::{
    Scene, SceneLabel, SceneLink, SceneNode, SceneOptions, DEFAULT_BACKGROUND,
    DEFAULT_LINK_CURVATURE, DEFAULT_TEXT_COLOR, LABEL_HEIGHT, MAX_LABELS, NODE_RADIUS,
};
pub use worker::{SimWorker, SnapshotRequest, WorkerSnapshot};
