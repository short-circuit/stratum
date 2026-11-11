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
    @location(0) @interpolate(flat) index: u32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) instance_position: vec3<f32>,
    @location(3) instance_color: vec4<f32>,
    @location(4) instance_radius: f32,
    @location(5) instance_index: u32,
) -> VSOut {
    let world = instance_position + position * instance_radius;
    var out: VSOut;
    out.clip = u.view_proj * vec4<f32>(world, 1.0);
    out.index = instance_index;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let index = in.index;
    let red = f32(index & 0xFFu) / 255.0;
    let green = f32((index >> 8u) & 0xFFu) / 255.0;
    let blue = f32((index >> 16u) & 0xFFu) / 255.0;
    return vec4<f32>(red, green, blue, 1.0);
}
