import super::common::CameraUniform;
import super::common::shadow_map;
import super::common::frag_pos_from_ray;
import super::textures;
import super::textures::TextureType;

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

var<immediate> texture_type: TextureType;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

struct InstanceInput {
    @location(4) position: vec3<f32>,
    @location(5) color_alpha: f32,
    @location(6) model_matrix_0: vec4<f32>,
    @location(7) model_matrix_1: vec4<f32>,
    @location(8) model_matrix_2: vec4<f32>,
    @location(9) model_matrix_3: vec4<f32>,
    @location(12) screen_space: u32, // question: should the location be universal across
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) pos_from_light: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    var out: VertexOutput;

    let model_matrix = mat4x4<f32>(
            pos.model_matrix_0,
            pos.model_matrix_1,
            pos.model_matrix_2,
            pos.model_matrix_3,
    );
    let model_position = model_matrix * vec4(model.position, 0.0, 1.0);
    let ratio_fixed_modelpos = vec4(model_position.xy * vec2(2.0*camera.inv_screen_size.x, 2.0*camera.inv_screen_size.y), model_position.z, 1.0);

    var coord = vec4<f32>(pos.position.xy, 0.0, 1.0);
    if pos.screen_space == 0 {
        coord = camera.view_proj * coord;
    } else {
        coord.x *= camera.inv_screen_size.x;
        coord.x = 2.0*(coord.x - 0.5);
        coord.y *= camera.inv_screen_size.y;
        coord.y = 2.0*(coord.y - 0.5) * -1.0;
    }
    out.color = vec4f(model.color.rgb, pos.color_alpha);
    out.uv = model.uv;
    out.clip_position = vec4<f32>(ratio_fixed_modelpos.xyz, 0.0) + vec4(coord.xyz/coord.w, 1.0);

    return out;    
}

@vertex
fn vs_main_sm(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    var out: VertexOutput;

    let clip_pos2d = model.uv * 2.0 - 1.0;

    let final_world_pos = (vec4<f32>(frag_pos_from_ray(camera, clip_pos2d), 1.0));

    out.clip_position = camera.view_proj * final_world_pos;

    // shift a bit ground shadows towards the light to create free contact shadow around building
    out.pos_from_light = camera.light_view_proj * vec4<f32>((final_world_pos.xy - 0.003), final_world_pos.z, 1.0);
    out.pos_from_light = vec4f(out.pos_from_light.xy * vec2f(0.5, -0.5) + 0.5, out.pos_from_light.zw);

    return out;
}

// Fragment shader
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var t_depth: texture_depth_2d;
@group(1) @binding(2)
var s_diffuse: sampler;
@group(1) @binding(3)
var s_compare: sampler_comparison;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

const tex_border_x: f32 = 0.01;

@fragment
fn fs_main_textured(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let tex_size = textureDimensions(t_diffuse);
    let tex_border_y = (tex_border_x * (f32(tex_size.x) / f32(tex_size.y)));
    if in.uv.x <= tex_border_x || in.uv.x >= 1.0 - tex_border_x || in.uv.y <= tex_border_y || in.uv.y >= 1.0 - tex_border_y {
         return vec4(1.0, 0.0, 0.0, 1.0);
    }
    if texture_type == textures::GENERAL_RGBA {
        return textureSample(t_diffuse, s_diffuse, in.uv.xy);
    } else if texture_type == textures::GENERAL_RGBA_R_NEG {
        let r = textureSample(t_diffuse, s_diffuse, in.uv.xy).r;
        return vec4f(vec3f(1.0 - r), 1.0);
    } else { // TextureType::DEPTH
        let depth = textureSample(t_depth, s_diffuse, in.uv.xy);
        return vec4f(vec3f(depth), 1.0);
    }
}

const weights = array<f32, 9>(
        1.0, 2.0, 1.0,
        2.0, 4.0, 2.0,
        1.0, 2.0, 1.0
    );

@fragment
fn fs_main_tex_storage(in: VertexOutput) -> @location(0) vec4<f32> {
    var result = 0.0;
    let texelSize = 1.0 / vec2f(textureDimensions(t_diffuse));

    var index = 0;

    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = in.uv + vec2f(f32(x), f32(y)) * texelSize;
            result += textureSample(t_diffuse, s_diffuse, offset).r * weights[index];
            index++;
        }
    }
    result = result / 16.0;

    return vec4f(0.0, 0.0, 0.0, result * max(0.0, 1.0 - camera.scale * 2.0));
}

@fragment
fn fs_main_sm(in: VertexOutput) -> @location(0) vec4<f32> {
    if(camera.scale >= 1.0) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    let shadow = shadow_map(t_depth, s_compare, in.pos_from_light.xy, 1.2, in.pos_from_light.z);

    return vec4(0.0, 0.0, 0.0, shadow * 0.5);
}