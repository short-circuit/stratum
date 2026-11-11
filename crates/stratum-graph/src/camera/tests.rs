//! Camera parity tests (tween, WASD, TrackballControls pan/dolly and framing).

use super::*;
use std::time::{Duration, Instant};

#[test]
fn focus_tween_reaches_the_node_at_500ms() {
    let camera = Camera::new(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO);
    let now = Instant::now();
    let mut tween = camera.focus_tween(Vec3::new(10.0, 20.0, -30.0), now);
    let mut camera = camera;
    assert!(tween.apply_at(now, &mut camera));
    assert!(tween.apply_at(now + Duration::from_millis(250), &mut camera));
    assert!(!tween.apply_at(now + FOCUS_DURATION, &mut camera));
    assert_eq!(camera.target, Vec3::new(10.0, 20.0, -30.0));
    assert_eq!(camera.position, Vec3::new(10.0, 20.0, 170.0));
}

#[test]
fn canceled_tween_does_not_move_the_camera() {
    let camera = Camera::new(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO);
    let now = Instant::now();
    let mut tween = camera.focus_tween(Vec3::new(100.0, 0.0, 0.0), now);
    tween.cancel();
    let mut camera = camera;
    assert!(!tween.apply_at(now + FOCUS_DURATION, &mut camera));
    assert_eq!(camera.position, Vec3::new(0.0, 0.0, 200.0));
}

#[test]
fn wasd_step_matches_the_shipped_horizontal_normalization() {
    let mut camera = Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
    camera.step(MoveKey::Forward);
    assert_eq!(camera.position, Vec3::new(0.0, 0.0, 9.2));
    assert_eq!(camera.target, Vec3::new(0.0, 0.0, -0.8));
    camera.step(MoveKey::Right);
    assert!((camera.position.x - 0.8).abs() < 1e-6);
    assert!((camera.target.x - 0.8).abs() < 1e-6);
}

#[test]
fn orbit_preserves_distance_and_pan_moves_both() {
    let mut camera = Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
    let before = camera.position.sub(camera.target).length();
    camera.orbit(0.5, 0.2);
    let after = camera.position.sub(camera.target).length();
    assert!((before - after).abs() < 1e-4);
    let position = camera.position;
    let target = camera.target;
    camera.pan(1.0, 2.0);
    assert!((camera.position.x - position.x).abs() > 1e-6);
    assert!((camera.target.y - target.y).abs() > 1e-6);
}

#[test]
fn pan_pixels_moves_camera_and_target_by_the_trackball_offset() {
    let mut camera = Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
    let before_position = camera.position;
    let before_target = camera.target;
    let distance = camera.position.sub(camera.target).length();
    let height = 600.0_f32;
    let dx = 100.0_f32;
    let expected = 2.0 * f64::from(distance) * f64::from((camera.fov_y * 0.5).tan())
        / f64::from(height)
        * PAN_SPEED
        * f64::from(dx);
    camera.pan_pixels(dx, 0.0, height);
    let position_delta = camera.position.sub(before_position);
    let target_delta = camera.target.sub(before_target);
    assert!((position_delta.sub(target_delta).length() as f64).abs() < 1e-4);
    assert!((position_delta.length() as f64 - expected).abs() < 1e-3);
    assert_eq!(camera.position.y, before_position.y);
}

#[test]
fn zoom_steps_follow_the_trackball_dolly_curve() {
    let mut camera = Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
    let before = camera.position.sub(camera.target).length();
    camera.zoom_steps(1.0);
    let zoomed_in = camera.position.sub(camera.target).length();
    let expected = before * 0.95_f32.powf(ZOOM_SPEED as f32);
    assert!((zoomed_in - expected).abs() < 1e-4);
    assert_eq!(camera.target, Vec3::ZERO);
    camera.zoom_steps(-1.0);
    let restored = camera.position.sub(camera.target).length();
    assert!((restored - before).abs() < 1e-4);
}

#[test]
fn pan_pixels_ignores_a_degenerate_viewport() {
    let mut camera = Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
    let before = camera;
    camera.pan_pixels(100.0, 50.0, 0.0);
    assert_eq!(camera, before);
    camera.pan_pixels(100.0, 50.0, f32::NAN);
    assert_eq!(camera, before);
}

#[test]
fn fit_to_positions_frames_every_input_position() {
    let positions: Vec<Vec3> = (0..64)
        .map(|index| {
            let column = (index % 8) as f32;
            let row = (index / 8) as f32;
            Vec3::new(column * 24.0 - 84.0, row * 18.0 - 63.0, column * 3.0 - 9.0)
        })
        .collect();
    let mut camera = Camera::default();
    camera.fit_to_positions(&positions);
    for position in &positions {
        let (x, y, _) = camera
            .project(*position, 1280, 720)
            .unwrap_or_else(|| panic!("position {position:?} was clipped"));
        assert!(
            (0.0..1280.0).contains(&x) && (0.0..720.0).contains(&y),
            "position {position:?} projected to ({x}, {y})"
        );
    }
    let centre = positions
        .iter()
        .fold(Vec3::ZERO, |sum, position| sum.add(*position))
        .scale(1.0 / positions.len() as f32);
    assert!((camera.target.sub(centre).length() - 0.0).abs() < 1e-2);
}

#[test]
fn fit_to_positions_ignores_an_empty_slice() {
    let mut camera = Camera::default();
    let before = camera;
    camera.fit_to_positions(&[]);
    assert_eq!(camera, before);
}

#[test]
fn shipped_camera_matches_the_3d_force_graph_placement() {
    let camera = Camera::shipped_for_nodes(24);
    let distance = (24.0_f32).cbrt() * CAMERA_DISTANCE2NODES_FACTOR;
    assert!((camera.position.z - distance).abs() < 1e-3);
    assert!(camera.position.x.abs() < 1e-6);
    assert!(camera.position.y.abs() < 1e-6);
    assert!(camera.position.z > 0.0);
    assert_eq!(camera.target, Vec3::ZERO);
    assert!(!camera.is_orthographic());
    assert!(camera.far >= distance * 4.0);

    let large = Camera::shipped_for_nodes(10_000);
    let expected = (10_000.0_f32).cbrt() * CAMERA_DISTANCE2NODES_FACTOR;
    assert!((large.position.z - expected).abs() < 1e-3);
    assert_eq!(large.target, Vec3::ZERO);
    assert!(!large.is_orthographic());

    let empty = Camera::shipped_for_nodes(0);
    assert!((empty.position.z - CAMERA_DISTANCE2NODES_FACTOR).abs() < 1e-6);
    assert_eq!(empty.target, Vec3::ZERO);
}

#[test]
fn orthographic_projection_and_rays_share_the_view_direction() {
    let mut camera = Camera::default();
    camera.set_orthographic(true);
    assert!(camera.is_orthographic());
    let (x, y, _) = camera
        .project(Vec3::ZERO, 1280, 720)
        .expect("origin visible");
    assert!((x - 640.0).abs() < 1e-3);
    assert!((y - 360.0).abs() < 1e-3);
    let (origin, direction) = camera.ray_from_screen(640.0, 360.0, 1280, 720);
    let forward = camera.target.sub(camera.position).normalized();
    assert!(direction.dot(forward) > 0.99);
    assert!(origin.sub(camera.position).length() < 1e-3);
    let (left_origin, _) = camera.ray_from_screen(0.0, 360.0, 1280, 720);
    assert!(left_origin.sub(origin).length() > 1e-3);
    camera.set_orthographic(false);
    assert!(!camera.is_orthographic());
}

#[test]
fn projection_and_rays_agree() {
    let camera = Camera::default();
    let (x, y, _) = camera
        .project(Vec3::ZERO, 1280, 720)
        .expect("origin visible");
    assert!((x - 640.0).abs() < 1e-3);
    assert!((y - 360.0).abs() < 1e-3);
    let (_, direction) = camera.ray_from_screen(640.0, 360.0, 1280, 720);
    let forward = camera.target.sub(camera.position).normalized();
    assert!(direction.dot(forward) > 0.99);
}
