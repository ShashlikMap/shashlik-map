import super::common::CameraUniform;
import super::common::pcf;

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
    let model_position = model_matrix * vec4(model.position.xyz, 1.0);
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

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>  {
    let gradient_koef = 0.5 + min(1.0, tanh(2.0 * in.world_position.z)) / 2.0;
    let diffuse_color = max(dot(in.world_normal, light_dir), 0.0);

    var shadow = 0.0;
    if((params & 2) > 0) {

        let currentDepth = in.pos_from_light.z;
        let projCoords = (in.pos_from_light.xy * vec2f(0.5, -0.5)) + 0.5;

        let texelSize = 1.2 / vec2f(textureDimensions(t_depth));

        let shadow_bias = 0.0007 * camera.scale;
        let depth_with_bias = currentDepth - shadow_bias;
        shadow = pcf(t_depth, s_compare, projCoords, texelSize, depth_with_bias);
    }

    let result_color = (ambient_color + (1.0 - shadow) * (diffuse_color)) * default_color;
    return vec4(result_color * gradient_koef, in.color_alpha);
}

@fragment
fn fs_main_g_buf(in: VertexOutput) -> GBuffer {

    // TODO use view matrix to calc z in VS instead of in.clip_position.z?
    return GBuffer(vec4(in.view_position.xyz, 1.0), vec4(in.view_normal.xyz, 1.0));
}