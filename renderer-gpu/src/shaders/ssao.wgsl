import super::common::CameraUniform;

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0) var ssao_texture: texture_storage_2d<rgba16float, write>;

@group(1) @binding(1) var positions: texture_2d<f32>;
@group(1) @binding(2) var normals: texture_2d<f32>;

@group(1) @binding(3) var noise: texture_2d<f32>;
@group(1) @binding(4) var kernel: texture_2d<f32>;

@compute @workgroup_size(8, 8, 1)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_coord = id.xy;
    let ssao_size = textureDimensions(ssao_texture);

    if(pixel_coord.x >= ssao_size.x || pixel_coord.y >= ssao_size.y) {
        return;
    }

    let screen_size = (1.0 / camera.inv_screen_size);
    compute_ssao(pixel_coord, vec2f(ssao_size), screen_size);
}

const radius: f32 = 0.5;

const samples: i32 = 16;

const noise_size: u32 = 4;

fn compute_ssao(pixel_coord: vec2<u32>, ssao_size: vec2f, screen_size: vec2f) {
    let pixel_mul = u32(round(screen_size.x / ssao_size.x));
    let loadedNormal = textureLoad(normals, pixel_mul * pixel_coord, 0).xyz;

    let normal = -normalize(loadedNormal);
    let fragPos = textureLoad(positions, pixel_mul * pixel_coord, 0).xyz;

    let noise_sample_coords = pixel_coord % vec2u(noise_size, noise_size);
    let noise_vec = textureLoad(noise, noise_sample_coords, 0).rgb;
    let tangent = normalize(noise_vec - normal * dot(noise_vec, normal));
    let bitangent = cross(normal, tangent);
    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    var valid = 0.0;
    for (var i = 0; i < samples; i++) {
        var kernel = textureLoad(kernel, vec2(i, 0), 0).rgb;
        let samplePos = fragPos + (TBN * kernel) * radius;
        if(abs(samplePos.z - fragPos.z) <= 0.0001) {
            continue;
        }
        let offset = camera.proj * vec4f(samplePos, 1.0);
        let ndcPos = offset.xy / offset.w;
        let uv = ndcPos * vec2f(0.5, -0.5) + vec2f(0.5);
        let screenCoord = vec2i(uv * screen_size);
        let center_coord = vec2i(pixel_mul * pixel_coord);
        if (all(screenCoord == center_coord)) {
            continue;
        }
        let sampleDepth = textureLoad(positions, screenCoord, 0).z;
        let depth_diff = abs(fragPos.z - sampleDepth);
        let rangeCheck = smoothstep(0.0, 1.0, radius / depth_diff);

        valid += 1.0;
        occlusion += select(0.0, 1.0, sampleDepth > samplePos.z + 0.03) * rangeCheck;
    }

    occlusion = occlusion / valid;
    textureStore(ssao_texture, pixel_coord, vec4f(occlusion, 0.0, 0.0, 0.0));
}