struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read_write> styles: vec4f;

@compute @workgroup_size(64)
fn compute_main(
) {
    styles.y = camera.p2_scale/100.0;
}