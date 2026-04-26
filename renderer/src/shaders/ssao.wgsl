import super::common::CameraUniform;
import super::common::frag_pos_from_ray;

@group(0) @binding(0) var ssao_texture: texture_storage_2d<rgba16float, write>;

@group(0) @binding(1) var normals: texture_2d<f32>;

@group(0) @binding(2) var positions: texture_2d<f32>;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@compute @workgroup_size(8, 8, 1)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_coord = id.xy;
    let ssao_size = textureDimensions(ssao_texture);

    if(pixel_coord.x >= ssao_size.x || pixel_coord.y >= ssao_size.y) {
        return;
    }

    compute_ssao(pixel_coord, vec2f(ssao_size));
}

const radius: f32 = 0.12;

const samples: i32 = 16;
const kernel = array<vec3f, samples>(
        vec3f(0.325931f, 0.331029f, 0.967709f),
                vec3f(-0.087517f, -0.739311f, 0.030716f),
                vec3f(0.504828f, 0.076937f, 0.227086f),
                vec3f(-0.639056f, 0.310719f, 0.751524f),
                vec3f(0.813696f, -0.674305f, 0.371703f),
                vec3f(-0.921673f, -0.361186f, 0.892556f),
                vec3f(-0.348158f, -0.513016f, 0.116095f),
                vec3f(-0.009523f, -0.776991f, 0.792030f),
                vec3f(0.212048f, -0.997931f, 0.000883f),
                vec3f(0.153850f, 0.098625f, 0.196745f),
                vec3f(-0.437783f, -0.308788f, 0.571267f),
                vec3f(-0.847969f, -0.691004f, 0.139359f),
                vec3f(0.279546f, -0.846539f, 0.317021f),
                vec3f(0.014279f, 0.465680f, 0.938725f),
                vec3f(-0.996073f, -0.600005f, 0.903233f),
                vec3f(-0.064112f, -0.017207f, 0.395765f)
    );

const noise_size: u32 = 8;
const noise: array<vec3f, 64> = array<vec3f, 64>(
        vec3f(0.498019f, -0.138723f, -0.719221f),
                vec3f(-0.715574f, -0.550533f, 0.907703f),
                vec3f(-0.292364f, -0.793055f, -0.087154f),
                vec3f(0.839407f, 0.397618f, 0.637048f),
                vec3f(0.563101f, -0.412131f, 0.134669f),
                vec3f(-0.384664f, 0.779189f, -0.012993f),
                vec3f(-0.106794f, -0.008540f, 0.107459f),
                vec3f(0.043171f, 0.337181f, 0.250799f),
                vec3f(0.194708f, -0.198697f, -0.550653f),
                vec3f(0.810833f, -0.350475f, -0.875343f),
                vec3f(-0.536346f, 0.644168f, 0.690995f),
                vec3f(-0.801291f, -0.420952f, 0.550404f),
                vec3f(-0.835689f, 0.374756f, 0.895130f),
                vec3f(0.399514f, 0.756081f, 0.290326f),
                vec3f(-0.017290f, -0.261606f, -0.852619f),
                vec3f(0.028135f, -0.004988f, -0.145844f),
                vec3f(0.925236f, 0.670672f, -0.595862f),
                vec3f(-0.181731f, -0.235191f, 0.022157f),
                vec3f(0.634719f, -0.678178f, -0.466862f),
                vec3f(-0.166032f, -0.959113f, 0.912086f),
                vec3f(0.294799f, 0.694731f, 0.968655f),
                vec3f(-0.756248f, -0.129110f, -0.775143f),
                vec3f(-0.145545f, -0.614639f, -0.328204f),
                vec3f(0.521186f, -0.620808f, -0.675555f),
                vec3f(0.454221f, 0.385564f, -0.770635f),
                vec3f(0.377082f, -0.186898f, -0.021890f),
                vec3f(-0.227836f, 0.043232f, 0.384662f),
                vec3f(0.679012f, 0.346184f, 0.304211f),
                vec3f(-0.410387f, -0.579056f, -0.524389f),
                vec3f(0.809811f, 0.373289f, -0.852437f),
                vec3f(0.362048f, 0.758220f, 0.796465f),
                vec3f(0.723425f, 0.879283f, -0.528698f),
                vec3f(0.897106f, 0.209561f, -0.966730f),
                vec3f(0.181903f, 0.285992f, -0.649164f),
                vec3f(-0.350314f, -0.119617f, 0.170969f),
                vec3f(0.100983f, -0.231861f, -0.752096f),
                vec3f(0.544564f, -0.474695f, -0.134790f),
                vec3f(-0.201528f, 0.143371f, 0.237068f),
                vec3f(0.264013f, -0.043871f, 0.426481f),
                vec3f(0.190253f, -0.261845f, 0.534904f),
                vec3f(0.562281f, 0.998846f, -0.138655f),
                vec3f(-0.874622f, 0.502285f, -0.407575f),
                vec3f(0.725221f, 0.412502f, -0.692343f),
                vec3f(0.842110f, -0.461721f, 0.175263f),
                vec3f(-0.464554f, -0.526278f, -0.224698f),
                vec3f(0.401873f, -0.349921f, -0.918575f),
                vec3f(0.072981f, 0.877680f, -0.029451f),
                vec3f(0.101450f, -0.776793f, 0.081240f),
                vec3f(-0.129580f, 0.219344f, 0.316880f),
                vec3f(-0.897223f, 0.997737f, 0.664348f),
                vec3f(-0.589849f, -0.544437f, -0.236715f),
                vec3f(-0.728900f, 0.750518f, -0.507394f),
                vec3f(-0.247685f, -0.076829f, -0.503319f),
                vec3f(-0.891851f, -0.052599f, -0.822399f),
                vec3f(-0.465307f, -0.533684f, 0.515048f),
                vec3f(-0.571649f, 0.130919f, -0.340096f),
                vec3f(-0.381016f, -0.895360f, -0.876841f),
                vec3f(0.993966f, 0.397833f, 0.103650f),
                vec3f(-0.355190f, -0.424137f, 0.409542f),
                vec3f(0.193815f, -0.830927f, -0.828270f),
                vec3f(-0.068930f, 0.543515f, -0.726717f),
                vec3f(-0.014475f, 0.347422f, -0.839274f),
                vec3f(-0.064014f, -0.210524f, 0.213194f),
                vec3f(-0.016824f, 0.526055f, 0.373054f)
    );

fn compute_ssao(pixel_coord: vec2<u32>, ssao_size: vec2f) {
    let loadedNormal = textureLoad(normals, pixel_coord, 0).xyz;
    if(loadedNormal.y > 0.0) {
        return;
    }
    var normal = loadedNormal;

    var fragPos = vec3f(0.0, 0.0, 0.0);
    if(loadedNormal.x == 0.0 && loadedNormal.y == 0.0 && loadedNormal.z == 0.0) {
        let uv_coord = (vec2f(pixel_coord.xy) * camera.inv_screen_size) * 2.0 - 1.0;
        fragPos = frag_pos_from_ray(camera, uv_coord);
        fragPos = (camera.view * vec4f(fragPos, 1.0)).xyz;
        normal = normalize((camera.view_tr_inv * vec4f(0.0, 0.0, 1.0, 1.0)).xyz);
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
        let dist_scale = f32(i) / f32(samples);
        let samplePos = fragPos + (TBN * (kl * lerp(0.1, 1.0, dist_scale * dist_scale))) * radius;

        let viewSampleDir = normalize(samplePos - fragPos);
        if(max(dot(normal, viewSampleDir), 0.0) == 0.0) {
            continue;
        }

        let offset = camera.proj * vec4f(samplePos, 1.0);
        let ndcPos = offset.xy / offset.w;
        let uv = ndcPos * vec2f(0.5, -0.5) + vec2f(0.5);
        let screenCoord = vec2i(uv * ssao_size);

        let sampleDepth = textureLoad(positions, screenCoord, 0).z;

        let rangeCheck = smoothstep(0.1, 1.0, (radius) / abs(fragPos.z - sampleDepth));

        occlusion += select(0.0, 1.0, sampleDepth >= samplePos.z + 0.05) * rangeCheck;
    }

    if(occlusion > 0.0) {
        occlusion = occlusion / f32(samples);
        textureStore(ssao_texture, pixel_coord, vec4f(occlusion, 0.0, 0.0, 0.0));
    }
}

fn lerp(a: f32, b: f32, f:f32) -> f32 {
    return a + f * (b - a);
}