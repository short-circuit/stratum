struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    viewport: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VSOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) start: vec3<f32>,
    @location(1) end: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) width: f32,
) -> VSOut {
    let midpoint = (start + end) * 0.5;
    let direction = normalize(end - start);
    var side = cross(direction, normalize(u.camera_pos.xyz - midpoint));
    if (length(side) < 0.0001) {
        side = normalize(u.camera_up.xyz);
    }
    side = normalize(side) * max(width, 0.05) * 0.5;
    var world: vec3<f32>;
    if (vertex_index == 0u) {
        world = start + side;
    } else if (vertex_index == 1u) {
        world = start - side;
    } else if (vertex_index == 2u) {
        world = end + side;
    } else if (vertex_index == 3u) {
        world = start - side;
    } else if (vertex_index == 4u) {
        world = end - side;
    } else {
        world = end + side;
    }
    var out: VSOut;
    out.clip = u.view_proj * vec4<f32>(world, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return in.color;
}
