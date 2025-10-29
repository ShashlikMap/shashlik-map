// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    ratio: f32,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct InstanceInput {
    @location(3) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    var out: VertexOutput;
    var modelpos = model.position + pos.position;
    var modelnormal = model.normal;
    out.world_position = modelpos;
    out.world_normal = modelnormal;
    out.clip_position = camera.view_proj * vec4<f32>(modelpos, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3(0.0, 0.0, 20.0) - in.world_position);

    let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse_color = vec4(1.0, 1.0, 1.0, 1.0) * diffuse_strength;

    return vec4(0.5, 0.5, 0.5, 1.0) * diffuse_color + vec4(0.1, 0.1, 0.1, 1.0);
}