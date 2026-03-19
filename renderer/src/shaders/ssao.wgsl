@group(0) @binding(0) var ssao_texture: texture_storage_2d<r32float, write>;

@group(0) @binding(1) var normals: texture_2d<f32>;

@group(0) @binding(2) var positions: texture_2d<f32>;

@group(0) @binding(3) var depth: texture_depth_2d;

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

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

fn hash33_vec3f(p: vec3f) -> vec3f {
    var v = bitcast<vec3u>(p);

    v = v * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v = v ^ (v >> vec3u(16u));
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;

    let res = vec3f(v) * (1.0 / 4294967295.0);
    return res * 2.0 - 1.0;
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
    if(pixel_coord.x >= u32(1.0 / camera.inv_screen_size.x) || pixel_coord.y >= u32(1.0 / camera.inv_screen_size.y)) {
        return;
    }

    var normal = -normalize(textureLoad(normals, pixel_coord, 0).xyz);

    if(normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0) {
        return;
    }

    let fragPos = textureLoad(positions, pixel_coord, 0).xyz;

//    let randomVec = normalize(camera.view_tr_inv * vec4f(hash_noise(vec2f(pixel_coord)), 1.0)).xyz;
//    let tangent = normalize(randomVec - normal * dot(randomVec, normal));
//    let bitangent = cross(normal, tangent);
//    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0; i < 8; i++) {
        var samplePos = (hash33_vec3f(fragPos) * get_sample_vector(u32(i), 8).y);
        samplePos = fragPos + samplePos * radius;

        let viewSampleDir = normalize(samplePos - fragPos);
        let NdotS = max(dot(normal, viewSampleDir), 0.0);
        if(NdotS == 0.0) {
            continue;
        }

        let offset2 = camera.proj * vec4f(samplePos, 1.0);
        let ndcPos = offset2.xy / offset2.w;
        let uv = ndcPos * vec2f(0.5, -0.5) + vec2f(0.5);
        let screenCoord = vec2i(uv / camera.inv_screen_size);

        let sampleDepth = textureLoad(positions, screenCoord, 0).z;

        var rangeCheck = smoothstep(0.1, 1.0, (radius) / abs(fragPos.z - sampleDepth));
        var aa = select(0.0, 1.0, sampleDepth > samplePos.z + 0.02) * rangeCheck * NdotS;
        occlusion += aa;
    }

    occlusion = (occlusion / 8.0);
    textureStore(ssao_texture, pixel_coord, vec4f(occlusion, 0.0, 0.0, 0.0));
}