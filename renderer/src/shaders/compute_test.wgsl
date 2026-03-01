struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct IndirectArgs {
    vertexCount: u32,
    instanceCount: atomic<u32>,
    reserved0: u32,
    reserved1: u32,
    reserved2: u32,
}

struct DotInput {
    position: vec3<f32>,
    color_alpha: f32,
}

@group(1) @binding(0)
var<storage, read_write> dots: array<DotInput>;

@group(1) @binding(1)
var<storage, read_write> culled: array<u32>;

@group(1) @binding(2)
var<storage, read_write> args: IndirectArgs;

@compute @workgroup_size(64)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let camera_scale = max(camera.scale, 0.25);
    let p2_scale = camera.p2_scale;

    let i = id.x;

    if(i % u32(p2_scale) != 0) {
        return;
    }

    let qqw = camera.view_proj * vec4f(dots[i].position, 1.0);
    let qq = qqw.xy / qqw.w;
    if qq.x < -1.0 || qq.x > 1.0 || qq.y < -1.0 || qq.y > 1.0 {
        return;
    }

    var ca = 1.0;
    if(i % (u32(p2_scale) * 2) != 0) {
        if(u32(p2_scale) == 1) {
            ca = 2.0 * (1.0 - camera_scale);
        } else {
            ca = 2.0 * (p2_scale - camera_scale) / p2_scale;
        }
    }

    if ca > 0.0 {
        dots[i].color_alpha = ca;
        args.vertexCount = 6;
        let culledIndex = atomicAdd(&args.instanceCount, 1u);
        culled[culledIndex] = i;
    }
}