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
    @location(0) color_alpha: f32,
}

@vertex
fn vs_main(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    var out: VertexOutput;

    let model_matrix = mat4x4<f32>(
            pos.model_matrix_0,
            pos.model_matrix_1,
            pos.model_matrix_2,
            pos.model_matrix_3,
    );
    let model_position = model_matrix * vec4(model.position.xyz, 1.0);
    let ratio_fixed_modelpos = vec4(model_position.x, model_position.y * camera.ratio, model_position.z, 1.0);

    out.color_alpha = pos.color_alpha;

    let coord = camera.view_proj * vec4<f32>(pos.position.xy, 0.0, 1.0);

    out.clip_position = vec4<f32>(ratio_fixed_modelpos.xyz * 0.00005, 0.0) + vec4(coord.xyz/coord.w, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 0.0, 1.0);
}