// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    ratio: f32,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct InstanceInput {
    @location(4) position: vec3<f32>,
    @location(5) color_alpha: f32,
    @location(6) model_matrix_0: vec4<f32>,
    @location(7) model_matrix_1: vec4<f32>,
    @location(8) model_matrix_2: vec4<f32>,
    @location(9) model_matrix_3: vec4<f32>,
    @location(10) bbox: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) color_alpha: f32,
}

@vertex
fn vs_main(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
            pos.model_matrix_0,
            pos.model_matrix_1,
            pos.model_matrix_2,
            pos.model_matrix_3,
    );
    let model_position = model_matrix * vec4(model.position.xyz, 1.0);
    var out: VertexOutput;
    var modelpos = model_position.xyz + pos.position;
    var modelnormal = model.normal;
    out.world_position = modelpos;
    out.world_normal = modelnormal;
    out.color_alpha = pos.color_alpha;
    out.clip_position = camera.view_proj * vec4<f32>(modelpos, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3(0.0, 0.0, 20.0) - in.world_position);

    let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse_color = vec4(1.0, 1.0, 1.0, 1.0) * diffuse_strength;

    return vec4(0.5, 0.5, 0.5, in.color_alpha) * diffuse_color + vec4(0.1, 0.1, 0.1, in.color_alpha);
}