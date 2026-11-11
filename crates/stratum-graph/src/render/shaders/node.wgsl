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
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
    @location(2) color: vec4<f32>,
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
    out.normal = normal;
    out.world = world;
    out.color = instance_color;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let light_dir = normalize(vec3<f32>(0.35, 0.8, 0.55));
    let diffuse = max(dot(n, light_dir), 0.0);
    let rim = pow(1.0 - max(dot(n, normalize(u.camera_pos.xyz - in.world)), 0.0), 3.0);
    let color = in.color.rgb * (0.32 + 0.68 * diffuse) + vec3<f32>(0.08, 0.10, 0.12) * rim;
    return vec4<f32>(color, in.color.a);
}
