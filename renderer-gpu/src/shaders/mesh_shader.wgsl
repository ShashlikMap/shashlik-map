import super::mesh_shader_common::{VertexInput, InstanceInput, VertexOutput};
import super::common::CameraUniform;
import super::common::shadow_map;

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

var<immediate> params: u32;

@vertex
fn vs_main(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
            pos.model_matrix_0,
            pos.model_matrix_1,
            pos.model_matrix_2,
            pos.model_matrix_3,
    );
    var model_position = model_matrix * vec4(model.position.xyz, 1.0);
    model_position.z = model_position.z * camera.scale_2d_3d;

    var out: VertexOutput;
    var modelpos = model_position.xyz + pos.position;

    var modelnormal = model.normal;
    // TODO
    modelnormal.z = -abs(modelnormal.z);
    out.world_normal = -modelnormal;

    out.color_alpha = pos.color_alpha;

    // check scale_2d_3d, when geometry is flat height = 1.0 gives a correct value for gradient in fragment shader
    if(modelpos.z > 0.0 || camera.scale_2d_3d == 0.0) {
        // technically, normalized z coord
        out.height = 1.0;
    }

    out.clip_position = camera.view_proj * vec4<f32>(modelpos, 1.0);

    // calc pos_from_light only if shadows pass, otherwise allow g-buf data if not shadows
    if((params & 2) > 0) {
        out.pos_from_light = camera.light_view_proj * vec4<f32>(modelpos, 1.0);
        out.pos_from_light = vec4f(out.pos_from_light.xy * vec2f(0.5, -0.5) + 0.5, out.pos_from_light.zw);
    } else {
        out.view_position = (camera.view * vec4f(modelpos, 1.0)).xyz;
        out.view_normal = (camera.view_tr_inv * vec4f(modelnormal, 1.0)).xyz;
    }

    return out;
}

const light_dir = normalize(vec3(0.84, 1.12, 1.42));

const sun_color = vec3<f32>(1.0, 0.98, 0.94);
const ambient_color = vec3<f32>(0.86, 0.90, 0.96) * 0.7;

const wall_color = vec3<f32>(0.9639, 0.9555, 0.9387);
const roof_color = vec3<f32>(0.843, 0.835, 0.816);

@group(1) @binding(0)
var t_depth: texture_depth_2d;
@group(1) @binding(1)
var s_compare: sampler_comparison;

const dither_strength = 2.0 / 255.0;
// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>  {
    let diffuse_factor = max(dot(in.world_normal, light_dir), 0.15);

    var shadow = 0.0;
    if((params & 2) > 0) {
        let currentDepth = in.pos_from_light.z;
        let projCoords = in.pos_from_light.xy;
        let shadow_bias = 0.0007 * camera.scale;
        let depth_with_bias = currentDepth - shadow_bias;

        shadow = shadow_map(t_depth, s_compare, projCoords, 1.2, depth_with_bias);
    }

    var base_color = roof_color;
    if (in.world_normal.z < 0.8) {
        // gradient only for walls
        let gradient_koef_ground = min(1.0, (0.9 + in.height * 7.0));
        let gradient_koef_walls = 0.85 + in.height * 0.15;
        base_color = wall_color * gradient_koef_walls * gradient_koef_ground;
    }

    let direct_sun = sun_color * (0.5 * diffuse_factor) * (1.0 - shadow * 0.3);
    let total_lighting = ambient_color + direct_sun;
    let result_color = total_lighting * base_color;

    let noise = fract(sin(dot(in.clip_position.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let final_color = (result_color) + (noise - 0.5) * dither_strength;

    return vec4(final_color, in.color_alpha);
}