struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    view_proj_inv: mat4x4<f32>,
    view_tr_inv: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
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
    @location(0) color_alpha: f32,
    @location(1) uv: vec2<f32>,
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
    let model_position = model_matrix * vec4(model.position.xyz, 1.0);
    let ratio_fixed_modelpos = vec4(model_position.xy * vec2(2.0*camera.inv_screen_size.x, 2.0*camera.inv_screen_size.y), model_position.z, 1.0);

    out.color_alpha = pos.color_alpha;

    var coord = vec4<f32>(pos.position.xy, 0.0, 1.0);
    if pos.screen_space == 0 {
        coord = camera.view_proj * coord;
    } else {
        coord.x *= camera.inv_screen_size.x;
        coord.x = 2.0*(coord.x - 0.5);
        coord.y *= camera.inv_screen_size.y;
        coord.y = 2.0*(coord.y - 0.5) * -1.0;
    }
    out.uv = model.uv;
    out.clip_position = vec4<f32>(ratio_fixed_modelpos.xyz, 0.0) + vec4(coord.xyz/coord.w, 1.0);
    return out;
}

// Fragment shader
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var t_depth: texture_depth_2d;
@group(1) @binding(2)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 0.0, in.color_alpha);
}

const tex_border_x: f32 = 0.02;

@fragment
fn fs_main_textured(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
//    let tex_size = textureDimensions(t_diffuse);
//    let tex_border_y = (tex_border_x * (f32(tex_size.x) / f32(tex_size.y)));
//    if in.uv.x <= tex_border_x || in.uv.x >= 1.0 - tex_border_x || in.uv.y <= tex_border_y || in.uv.y >= 1.0 - tex_border_y {
//         return vec4(1.0, 0.0, 0.0, 1.0);
//    }
//    return textureSample(t_diffuse, s_diffuse, in.uv.xy);
    let gg = textureSample(t_depth, s_diffuse, in.uv.xy);
    return vec4(gg, gg, gg, 1.0);
}

@fragment
fn fs_main_tex_storage(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_coord = vec2i(floor(in.clip_position.xy * 0.5));
    var result = 0.0;
    result += textureLoad(t_diffuse, pixel_coord + vec2i(0, 0), 0).r;
    result += textureLoad(t_diffuse, pixel_coord + vec2i(1, 0), 0).r;
    result += textureLoad(t_diffuse, pixel_coord + vec2i(0, 1), 0).r;
    result += textureLoad(t_diffuse, pixel_coord + vec2i(1, 1), 0).r;
    return vec4f(0.0, 0.0, 0.0, result / 4.0);
}

// FAKE for compatibility
@fragment
fn fs_main_g_buf(in: VertexOutput) -> @location(0) vec4<f32>  {
    return vec4(1.0, 1.0, 1.0, 1.0);
}