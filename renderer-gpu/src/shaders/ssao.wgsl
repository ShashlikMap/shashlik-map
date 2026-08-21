import super::common::CameraUniform;
import super::common::frag_pos_from_ray;

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

const radius: f32 = 0.35;

const samples: i32 = 16;

const noise_size: u32 = 4;

fn compute_ssao(pixel_coord: vec2<u32>, ssao_size: vec2f, screen_size: vec2f) {
    let pixel_mul = u32(screen_size.x / ssao_size.x);
    let loadedNormal = textureLoad(normals, pixel_mul * pixel_coord, 0).xyz;
    if(loadedNormal.y > 0.0) {
        return;
    }
    var normal = loadedNormal;

    var fragPos = vec3f(0.0, 0.0, 0.0);
    if(loadedNormal.x == 0.0 && loadedNormal.y == 0.0 && loadedNormal.z == 0.0) {
        let uv_coord = (vec2f(pixel_mul * pixel_coord.xy) * camera.inv_screen_size) * 2.0 - 1.0;
        fragPos = frag_pos_from_ray(camera, uv_coord);
        fragPos = (camera.view * vec4f(fragPos, 1.0)).xyz;
        normal = normalize((camera.view_tr_inv * vec4f(0.0, 0.0, 1.0, 1.0)).xyz);
    } else {
        normal = -normalize(loadedNormal);
        fragPos = textureLoad(positions, pixel_mul * pixel_coord, 0).xyz;
    }

    let noise_sample_coords = pixel_coord % vec2u(noise_size, noise_size);
    let noise_vec = textureLoad(noise, noise_sample_coords, 0).rgb;
    let randomVec = noise_vec;
    let tangent = normalize(randomVec - normal * dot(randomVec, normal));
    let bitangent = cross(normal, tangent);
    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    var valid = 0.0;
    for (var i = 0; i < samples; i++) {
        var kl = textureLoad(kernel, vec2(i, 0), 0).rgb;
        let dist_scale = f32(i) / f32(samples);

        let samplePos = fragPos + (TBN * (kl * mix(0.1, 1.0, dist_scale * dist_scale))) * radius;

        let viewSampleDir = normalize(samplePos - fragPos);
        let ndots = max(dot(normal, viewSampleDir), 0.0);
        if(ndots == 0.0) {
            continue;
        }

        let offset = camera.proj * vec4f(samplePos, 1.0);
        let ndcPos = offset.xy / offset.w;
        let uv = ndcPos * vec2f(0.5, -0.5) + vec2f(0.5);
        let screenCoord = vec2i(uv * screen_size);
        let sampleDepth = textureLoad(positions, screenCoord, 0).z;
        let rangeCheck = smoothstep(0.0, 1.0, (radius) / abs(fragPos.z - sampleDepth));

        valid += 1.0;
        occlusion += select(0.0, 1.0, sampleDepth > samplePos.z + 0.025) * rangeCheck;
    }

    occlusion = occlusion / valid;
    textureStore(ssao_texture, pixel_coord, vec4f(occlusion, 0.0, 0.0, 0.0));
}