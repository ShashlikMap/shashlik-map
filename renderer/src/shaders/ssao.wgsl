@group(0) @binding(0) var ssao_texture: texture_storage_2d<r32float, write>;

@group(0) @binding(1) var normals: texture_2d<f32>;

@group(0) @binding(2) var positions: texture_2d<f32>;

@group(0) @binding(3) var depth: texture_depth_2d;

struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    view_tr_inv: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

fn random(st: vec2<f32>) -> f32 {
    return fract(sin(dot(st, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

fn hash_noise(p: vec2<f32>) -> vec3<f32> {
    let r = fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let g = fract(sin(dot(p, vec2<f32>(93.11, 23.14))) * 12345.6789);
    return normalize(vec3<f32>(r * 2.0 - 1.0, g * 2.0 - 1.0, 0.0));
}

fn get_sample_vector(index: u32, total_samples: f32) -> vec3<f32> {
    let phi = 2.0 * 3.14159 * (f32(index) * 0.61803398875); // Golden ratio
    let cos_theta = 1.0 - (f32(index) + 0.5) / total_samples;
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);

    // Returns a sample in a hemisphere oriented toward +Z (tangent space)
    return vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
}

const radius: f32 = 0.1;

@compute @workgroup_size(8, 8, 1)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_coord = id.xy;

    let fragPos = textureLoad(positions, pixel_coord, 0).xyz;
    let normal = normalize(textureLoad(normals, pixel_coord, 0).xyz);
    if(normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0) {
        textureStore(ssao_texture, pixel_coord, vec4f(0.0, 0.0, 0.0, 0.0));
        return;
    }

    let randomVec = normalize(hash_noise(vec2f(pixel_coord)));

    let tangent = normalize(randomVec - normal * dot(randomVec, normal));
    let bitangent = cross(normal, tangent);
    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0; i < 64; i++) {
        var samplePos = TBN * normalize(get_sample_vector(u32(i), 64.0));
        samplePos = fragPos + samplePos * radius;

        let viewSampleDir = normalize(samplePos - fragPos);
        let NdotS = max(dot(normal, viewSampleDir), 0.0);

        let offset2 = camera.proj * vec4f(samplePos, 1.0);
        let ndcPos = offset2.xy / offset2.w;
        let uv = ndcPos * vec2f(0.5, -0.5) + vec2f(0.5);
        let screenCoord = vec2i(uv / camera.inv_screen_size);

        let sampleDepth = textureLoad(positions, screenCoord, 0).z;

//        var rangeCheck = smoothstep(0.2, 1.0, (radius) / abs(fragPos.z - sampleDepth));
        occlusion += select(1.0, 0.0, sampleDepth <= samplePos.z + 0.0);
    }

    occlusion = (occlusion / 64.0);
    textureStore(ssao_texture, pixel_coord, vec4f(occlusion, 0.0, 0.0, 0.0));
}