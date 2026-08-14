import super::mesh_shader_common::{VertexOutput};

struct GBuffer {
    @location(0) positions: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

@fragment
fn fs_main_g_buf(in: VertexOutput) -> GBuffer {
    // TODO use view matrix to calc z in VS instead of in.clip_position.z?
    return GBuffer(vec4(in.view_position.xyz, 1.0), vec4(in.view_normal.xyz, 1.0));
}