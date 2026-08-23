import super::mesh_shader_common::{VertexOutput, InstanceInput};
import super::common::CameraUniform;
import super::common::frag_pos_from_ray;

struct GBuffer {
    @location(0) positions: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

const positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0,  1.0),
    vec2<f32>( 3.0,  1.0),
    vec2<f32>(-1.0, -3.0)
);

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    @builtin(vertex_index) vertexIndex: u32,
) -> VertexOutput {
    let clip_pos2d = positions[vertexIndex];
    let final_world_pos = (vec4<f32>(frag_pos_from_ray(camera, clip_pos2d), 1.0));

    var out: VertexOutput;
    out.view_position = (camera.view * vec4f(final_world_pos.xyz, 1.0)).xyz;
    out.view_normal = (camera.view_tr_inv * vec4f(0.0, 0.0, -1.0, 1.0)).xyz;
    out.clip_position = camera.view_proj * final_world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> GBuffer {
    return GBuffer(vec4(in.view_position.xyz, 1.0), vec4(in.view_normal.xyz, 1.0));
}