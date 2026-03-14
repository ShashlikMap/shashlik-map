@group(0) @binding(0) var ssao_texture: texture_storage_2d<r32float, write>;

@group(0) @binding(1) var normals: texture_2d<f32>;

@group(0) @binding(2) var positions: texture_2d<f32>;

@group(0) @binding(3) var depth: texture_2d<f32>;

struct CameraUniform {
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

const randomVec: vec3f = vec3f(0.0, -1.0, 0.0);

fn random(st: vec2<f32>) -> f32 {
    return fract(sin(dot(st, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

@compute @workgroup_size(8, 8, 1)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_coord = id.xy;
    let normal = textureLoad(normals, pixel_coord, 0).xyz;
    let fragPos = textureLoad(positions, pixel_coord, 0).xyz;
    let tangent = normalize(randomVec - normal * dot(randomVec, normal));
    let bitangent = cross(normal, tangent);
    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0; i < 1; i++) {
        let v1 = random(f32(pixel_coord.x) * 2.0 * normal.xy);
        let v2 = random(f32(pixel_coord.y) * 3.0 * tangent.xy);
        let v3 = random(f32(pixel_coord.x) * 4.0 * bitangent.xy);
        var samplePos = TBN * vec3f(v1, v2, v3);
        samplePos = fragPos + samplePos * 0.5;
        let offset = camera.view_proj * vec4f(samplePos, 1.0);
//        let abc = ((offset.xyz / offset.w) * 0.5 + 0.5).xy;

        let sampleDepth = textureLoad(positions, vec2i(i32(offset.x), i32(offset.y)), 0).z;
//        textureStore(ssao_texture, vec2i(i32(offset.x), i32(offset.y)), vec4f(sampleDepth, 0.0, 0.0, 0.0));

        if(sampleDepth >= samplePos.z) {
            occlusion += 1.0;
        }
    }
    occlusion = (occlusion / 64.0);
}