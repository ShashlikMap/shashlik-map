import super::mesh_shader_common::{VertexInput, InstanceInput, VertexOutput};
import super::common::CameraUniform;

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

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
    var model_position = model_matrix * vec4(model.position.xyz, 1.0);
    model_position.z = model_position.z * camera.scale_2d_3d;

    var out: VertexOutput;
    var modelpos = model_position.xyz + pos.position;

    out.clip_position = camera.light_view_proj * vec4<f32>(modelpos, 1.0);
    return out;
}