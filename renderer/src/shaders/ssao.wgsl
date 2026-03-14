@group(0) @binding(0) var ssao_texture: texture_storage_2d<r32float, write>;

@group(0) @binding(1) var normals: texture_2d<f32>;

@group(0) @binding(2) var positions: texture_2d<f32>;

@compute @workgroup_size(8, 8, 1)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_coord = id.xy;
    let center = vec2f(256.0);
    let dist = distance(vec2f(pixel_coord), center);
    let stripe = dist / 32.0 % 2.0;
    let normal = textureLoad(normals, pixel_coord, 0);
    let pos = textureLoad(positions, pixel_coord, 0);
    let color = select(normal.r * pos.r, normal.g * pos.g, stripe < 1.0);
    textureStore(ssao_texture, pixel_coord, vec4f(color, 0.0, 0.0, 0.0));
}