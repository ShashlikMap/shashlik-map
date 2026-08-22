import super::mesh_shader_common::{VertexOutput, InstanceInput};
import super::common::CameraUniform;
import super::common::frag_pos_from_ray;

struct GBuffer {
    @location(0) positions: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    let clip_pos2d = model.uv * 2.0 - 1.0;
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