import super::common::CameraUniform;
import super::common::shadow_map;

// Vertex shader

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

var<immediate> params: u32;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct InstanceInput {
    @location(4) position: vec3<f32>,
    @location(5) color_alpha: f32,
    @location(6) model_matrix_0: vec4<f32>,
    @location(7) model_matrix_1: vec4<f32>,
    @location(8) model_matrix_2: vec4<f32>,
    @location(9) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) view_normal: vec3<f32>,
    @location(2) view_position: vec3<f32>,
    @location(3) world_normal: vec3<f32>,
    @location(4) world_position: vec3<f32>,
    @location(5) pos_from_light: vec4<f32>,
    @location(6) color_alpha: f32,
    @location(7) height: f32,
}

struct GBuffer {
    @location(0) positions: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

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
    out.world_position = modelpos;
    out.world_normal = -modelnormal;

    out.view_position = (camera.view * vec4f(modelpos, 1.0)).xyz;
    out.view_normal = (camera.view_tr_inv * vec4f(modelnormal, 1.0)).xyz;
    out.color_alpha = pos.color_alpha;
    out.pos_from_light = camera.light_view_proj * vec4<f32>(modelpos, 1.0);
    // check scale_2d_3d, when geometry is flat height = 1.0 gives a correct value for gradient in fragment shader
    if(modelpos.z > 0.0 || camera.scale_2d_3d == 0.0) {
        // technically, normalized z coord
        out.height = 1.0;
    }

    if((params & 1) > 0) {
        out.clip_position = out.pos_from_light;
    } else {
        out.clip_position = camera.view_proj * vec4<f32>(modelpos, 1.0);
    }
    return out;
}

const light_dir = normalize(vec3(0.5, 0.5, 0.6));
const default_color = vec3(0.4, 0.4, 0.4);
const ambient_color = vec3(0.65, 0.65, 0.65);

@group(1) @binding(0)
var t_depth: texture_depth_2d;
@group(1) @binding(1)
var s_compare: sampler_comparison;

const dither_strength = 2.0 / 255.0;
// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>  {
    let gradient_koef_ground = min(1.0, (0.9 + in.height * 7.0));
    let gradient_koef_walls = 0.85 + in.height * 0.15;
    let diffuse_color = max(dot(in.world_normal, light_dir), 0.0);

    var shadow = 0.0;
    if((params & 2) > 0) {

        let currentDepth = in.pos_from_light.z;
        let projCoords = (in.pos_from_light.xy * vec2f(0.5, -0.5)) + 0.5;
        let shadow_bias = 0.0007 * camera.scale;
        let depth_with_bias = currentDepth - shadow_bias;

        shadow = shadow_map(t_depth, s_compare, projCoords, 1.2, depth_with_bias);
    }

    let result_color = (ambient_color + (1.0 - shadow * 0.6) * (diffuse_color)) * default_color * gradient_koef_walls * gradient_koef_ground;

    let noise = fract(sin(dot(in.clip_position.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let final_color = (result_color) + (noise - 0.5) * dither_strength;

    return vec4(final_color, in.color_alpha);
}

@fragment
fn fs_main_g_buf(in: VertexOutput) -> GBuffer {

    // TODO use view matrix to calc z in VS instead of in.clip_position.z?
    return GBuffer(vec4(in.view_position.xyz, 1.0), vec4(in.view_normal.xyz, 1.0));
}