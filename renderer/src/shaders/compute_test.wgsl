struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read_write> styles: array<f32>;

@compute @workgroup_size(64)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let camera_scale = max(camera.scale, 0.25);
    let p2_scale = camera.p2_scale;

    let i = id.x;
    if(i % u32(p2_scale) != 0) {
        styles[i] = 0.0;
        return;
    }
    if(i % (u32(p2_scale) * 2) != 0) {
        if(u32(p2_scale) == 1) {
            styles[i] = 2.0 * (1.0 - camera_scale);
        } else {
            styles[i] = 2.0 * (p2_scale - camera_scale) / p2_scale;
        }
    }
}