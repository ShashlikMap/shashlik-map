struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    view_proj_inv: mat4x4<f32>,
    view_tr_inv: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct IndirectArgs {
    indexCount: u32,
    instanceCount: atomic<u32>,
    reserved0: u32,
    reserved1: u32,
    reserved2: u32,
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

@group(1) @binding(0)
var<storage, read_write> indirect_instances: array<InstanceInput>;

@group(1) @binding(1)
var<storage, read_write> culled: array<u32>;

@group(2) @binding(0)
var<storage, read_write> args: IndirectArgs;

@compute @workgroup_size(64)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;

    let p2_scale = camera.p2_scale;

    if(i % u32(p2_scale) != 0 || i >= arrayLength(&indirect_instances)) {
        return;
    }

    let screen_pos = camera.view_proj * vec4f(indirect_instances[i].position, 1.0);
    let ndc = screen_pos.xy / screen_pos.w;
    if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 {
        return;
    }

    var ca = 1.0;
    if(i % (u32(p2_scale) * 2) != 0) {
        let camera_scale = max(camera.scale, 0.25);
        if(u32(p2_scale) == 1) {
            ca = 2.0 * (1.0 - camera_scale);
        } else {
            ca = 2.0 * (p2_scale - camera_scale) / p2_scale;
        }
    }

    if ca > 0.0 {
        indirect_instances[i].color_alpha = ca;
        let culledIndex = atomicAdd(&args.instanceCount, 1u);
        culled[culledIndex] = i;
    }
}