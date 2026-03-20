@group(0) @binding(0) var ssao_texture: texture_storage_2d<r32float, read_write>;

@group(0) @binding(1) var normals: texture_2d<f32>;

@group(0) @binding(2) var positions: texture_2d<f32>;

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

@compute @workgroup_size(8, 8, 1)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_coord = id.xy;
    let ssao_size = 2.0* vec2f(1.0 / camera.inv_screen_size.x, 1.0 / camera.inv_screen_size.y);

    if(pixel_coord.x >= u32(ssao_size.x) || pixel_coord.y >= u32(ssao_size.y)) {
        return;
    }

    compute_ssao(pixel_coord, ssao_size);
}

const kernel = array<vec3f, 8>(
      vec3f(-0.988005f, -0.614986f, 0.277931f),
      vec3f(0.308289f, 0.470236f, 0.121804f),
      vec3f(-0.286518f, 0.318795f, 0.864876f),
      vec3f(-0.330144f, -0.274284f, 0.511152f),
      vec3f(0.477399f, -0.142303f, -0.593631f),
      vec3f(0.810853f, -0.212716f, 0.217826f),
      vec3f(-0.100139f, 0.818944f, 0.441590f),
      vec3f(0.752630f, -0.281644f, 0.447299f)
  );

const noise_size: u32 = 2;
const noise: array<vec3f, 8> = array<vec3f, 8>(
        vec3f(0.607190f, -0.312189f, 0.818114f),
        vec3f(0.156606f, -0.817238f, -0.217971f),
        vec3f(-0.417291f, 0.065830f, -0.060644f),
        vec3f(-0.732652f, -0.385451f, -0.551573f),
        vec3f(0.585352f, -0.433822f, 0.307726f),
        vec3f(0.635344f, 0.910413f, 0.477567f),
        vec3f(-0.147777f, -0.358135f, -0.246600f),
        vec3f(0.545311f, -0.709271f, -0.237195f)
    );

const samples: i32 = 11;
const radius: f32 = 0.2;
fn compute_ssao(pixel_coord: vec2<u32>, ssao_size: vec2f) {
    let loadedNormal = textureLoad(normals, pixel_coord, 0).xyz;
    if(loadedNormal.y > 0.0) {
        return;
    }
    var normal = loadedNormal;

    var fragPos = vec3f(0.0, 0.0, 0.0);
    if(loadedNormal.x == 0.0 && loadedNormal.y == 0.0 && loadedNormal.z == 0.0) {
        let u_coord = (f32(2 * pixel_coord.x) * camera.inv_screen_size.x) * 2.0 - 1.0;
        let v_coord = (f32(2 * pixel_coord.y) * camera.inv_screen_size.y) * 2.0 - 1.0;
        let near_world1 = camera.view_proj_inv * vec4f(u_coord, v_coord, 0.0, 1.0);
        let near_world = near_world1.xyz / near_world1.w;
        let far_world1 = camera.view_proj_inv * vec4f(u_coord, v_coord, 1.0, 1.0);
        let far_world = far_world1.xyz / far_world1.w;

        var u = -near_world.z / (far_world.z - near_world.z);
        if u < 0.0 {
            u = 1.0 - u;
        }
        fragPos = near_world + u * (far_world - near_world);
        fragPos = (camera.view * vec4f(fragPos, 1.0)).xyz;
        normal = -normalize((camera.view_tr_inv * vec4f(0.0, 0.0, 1.0, 1.0)).xyz);
    } else {
        normal = -normalize(loadedNormal);
        fragPos = textureLoad(positions, pixel_coord, 0).xyz;
    }


    let noise_sample_coords = pixel_coord % vec2u(noise_size, noise_size);
    let noise_vec = noise[noise_sample_coords.y * noise_size + noise_sample_coords.x];
    let randomVec = (camera.view * vec4f(noise_vec, 0.0)).xyz;
    let tangent = normalize(randomVec - normal * dot(randomVec, normal));
    let bitangent = cross(normal, tangent);
    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0; i < samples; i++) {
        var kl = kernel[i];
        kl.z = kl.z * 0.5 + 0.5;
        let dist_scale = f32(i) / f32(samples);
        let samplePos = fragPos + (TBN * (kl * lerp(0.1, 1.0, dist_scale * dist_scale))) * radius;

        let viewSampleDir = normalize(samplePos - fragPos);
        let NdotS = max(dot(normal, viewSampleDir), 0.0);
        if(NdotS == 0.0) {
            continue;
        }

        let offset2 = camera.proj * vec4f(samplePos, 1.0);
        let ndcPos = offset2.xy / offset2.w;
        let uv = ndcPos * vec2f(0.5, -0.5) + vec2f(0.5);
        let screenCoord = vec2i(uv / camera.inv_screen_size) / 2;

        let sampleDepth = textureLoad(positions, screenCoord, 0).z;

        let rangeCheck = smoothstep(0.1, 1.0, (radius) / abs(fragPos.z - sampleDepth));

        occlusion += select(0.0, 1.0, sampleDepth > samplePos.z + 0.015) * rangeCheck;
    }

    occlusion = occlusion / f32(samples);
    if(occlusion > 0.0) {
        textureStore(ssao_texture, pixel_coord, vec4f(occlusion, 0.0, 0.0, 0.0));
    }
}

fn lerp(a: f32, b: f32, f:f32) -> f32 {
    return a + f * (b - a);
}