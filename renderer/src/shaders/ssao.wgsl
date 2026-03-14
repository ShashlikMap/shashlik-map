@group(0) @binding(0) var ssao_texture: texture_storage_2d<r32float, write>;

@group(0) @binding(1) var normals: texture_2d<f32>;

@group(0) @binding(2) var positions: texture_2d<f32>;

@compute @workgroup_size(8, 8, 1)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_coord = id.xy;
    let normal = textureLoad(normals, pixel_coord, 0);
    if abs(normal.x) > 0.0 || abs(normal.y) > 0.0 || abs(normal.z) > 0.0 {
        textureStore(ssao_texture, pixel_coord, vec4f(0.0, 0.0, 0.0, 0.0));
    } else {
        textureStore(ssao_texture, pixel_coord, vec4f(1.0, 0.0, 0.0, 0.0));
    }
}