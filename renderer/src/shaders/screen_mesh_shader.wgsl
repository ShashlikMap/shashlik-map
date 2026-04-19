struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    view_proj_inv: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
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
@group(1) @binding(3)
var s_compare: sampler_comparison;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 0.0, in.color_alpha);
}

const tex_border_x: f32 = 0.02;

@fragment
fn fs_main_textured(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let tex_size = textureDimensions(t_diffuse);
    let tex_border_y = (tex_border_x * (f32(tex_size.x) / f32(tex_size.y)));
    if in.uv.x <= tex_border_x || in.uv.x >= 1.0 - tex_border_x || in.uv.y <= tex_border_y || in.uv.y >= 1.0 - tex_border_y {
         return vec4(1.0, 0.0, 0.0, 1.0);
    }
    return textureSample(t_diffuse, s_diffuse, in.uv.xy);
}

@fragment
fn fs_main_tex_storage(in: VertexOutput) -> @location(0) vec4<f32> {
    let f = fract(floor(in.clip_position.xy * 0.5) - 0.5);

    let values = textureGather(0, t_diffuse, s_diffuse, in.uv);
    let top = mix(values.w, values.z, f.x);
    let bottom = mix(values.x, values.y, f.x);
    return vec4f(0.0, 0.0, 0.0, mix(top, bottom, f.y));
}

@fragment
fn fs_main_sm(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_coord = in.clip_position.xy;
    let u_coord = (pixel_coord.x * camera.inv_screen_size.x) * 2.0 - 1.0;
    let v_coord = (pixel_coord.y * camera.inv_screen_size.y) * 2.0 - 1.0;
    let near_world1 = camera.view_proj_inv * vec4f(u_coord, v_coord, 0.0, 1.0);
    let near_world = near_world1.xyz / near_world1.w;
    let far_world1 = camera.view_proj_inv * vec4f(u_coord, v_coord, 1.0, 1.0);
    let far_world = far_world1.xyz / far_world1.w;

    var u = -near_world.z / (far_world.z - near_world.z);
    if u < 0.0 {
        u = 1.0 - u;
    }
    var fragPos = near_world + u * (far_world - near_world);
    // shift a bit ground shadows towards the light to create free contact shadow around building
    fragPos.x -= 0.006;
    fragPos.y -= 0.006;

    let pos_from_light = camera.light_view_proj * vec4<f32>(vec3f(fragPos), 1.0);
    let currentDepth = pos_from_light.z;
    let projCoords = (pos_from_light.xy * vec2f(0.5, -0.5)) + 0.5;

    let texelSize = 2.0 / vec2f(textureDimensions(t_depth));
    var shadow = 0.0;
    for (var xx = -1; xx <= 1; xx++) {
        for (var yy = -1; yy <= 1; yy++) {
            shadow += (textureSampleCompare(t_depth, s_compare, projCoords + vec2f(f32(xx), f32(yy)) * texelSize, currentDepth));
        }
    }
    shadow /= 9.0;
    shadow = shadow * 0.5;

    return vec4(0.0, 0.0, 0.0, shadow);
}

// FAKE for compatibility
@fragment
fn fs_main_g_buf(in: VertexOutput) -> @location(0) vec4<f32>  {
    return vec4(1.0, 1.0, 1.0, 1.0);
}