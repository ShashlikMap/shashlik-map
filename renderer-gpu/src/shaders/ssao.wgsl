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
    let loaded_normal = textureLoad(normals, pixel_mul * pixel_coord, 0).xyz;

    let normal = -normalize(loaded_normal);
    let frag_pos = textureLoad(positions, pixel_mul * pixel_coord, 0).xyz;

    let noise_sample_coords = pixel_coord % vec2u(noise_size, noise_size);
    let noise_vec = textureLoad(noise, noise_sample_coords, 0).rgb;
    let tangent = normalize(noise_vec - normal * dot(noise_vec, normal));
    let bitangent = cross(normal, tangent);
    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    var valid = 0.0;
    for (var i = 0; i < samples; i++) {
        var kernel = textureLoad(kernel, vec2(i, 0), 0).rgb;
        let sample_vector = TBN * kernel;

        let dot_bias = max(dot(normal, normalize(sample_vector)), 0.0);
        // fast exit if sample_vector is orthogonal to normal
        // TODO we can prepare kernel to prevent wasted samples
        if(dot_bias == 0.0) {
            continue;
        }
        let sample_pos = frag_pos + sample_vector * radius;

        let offset = camera.proj * vec4f(sample_pos, 1.0);
        let ndcPos = offset.xy / offset.w;
        let uv = ndcPos * vec2f(0.5, -0.5) + vec2f(0.5);
        let screen_coord = vec2i(uv * screen_size);
        let center_coord = vec2i(pixel_mul * pixel_coord);
        if (all(screen_coord == center_coord)) {
            continue;
        }
        let sample_depth = textureLoad(positions, screen_coord, 0).z;
        let depth_diff = abs(frag_pos.z - sample_depth);
        let range_check = smoothstep(0.0, 1.0, radius / depth_diff);

        valid += 1.0;
        occlusion += select(0.0, 1.0, sample_depth > sample_pos.z + 0.025) * range_check * dot_bias;
    }

    // valid potentially can be 0.0
    occlusion = select(0.0, occlusion / valid, valid > 0.0);
    textureStore(ssao_texture, pixel_coord, vec4f(occlusion, 0.0, 0.0, 0.0));
}