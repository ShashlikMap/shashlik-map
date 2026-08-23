import super::common::CameraUniform;

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
    @location(10) virtual_plane: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) view_normal: vec3<f32>,
    @location(2) view_position: vec3<f32>,
    @location(3) world_normal: vec3<f32>,
    @location(4) pos_from_light: vec4<f32>,
    @location(5) color_alpha: f32,
    @location(6) height: f32,
    @location(7) @interpolate(flat) virtual_plane: u32,
}