struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    viewport: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(1) var atlas: texture_2d<f32>;
@group(1) @binding(2) var atlas_sampler: sampler;

struct VSOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) anchor: vec3<f32>,
    @location(1) offset: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
) -> VSOut {
    let world = anchor + u.camera_right.xyz * offset.x + u.camera_up.xyz * offset.y;
    var out: VSOut;
    out.clip = u.view_proj * vec4<f32>(world, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let distance = textureSample(atlas, atlas_sampler, in.uv).r;
    let alpha = smoothstep(0.42, 0.58, distance);
    if (alpha <= 0.01) {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
