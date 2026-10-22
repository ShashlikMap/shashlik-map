import super::common::CameraUniform;
import super::common::camera;
import super::globe_common::GLOBE_SCALE;
import super::globe_common::GLOBE_R;
import super::globe_common::transform_to_globe_position;

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
    @location(13) @interpolate(flat) screen_space: u32, // question: should the location be universal across
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) uv_cube: vec3<f32>,
    @location(3) @interpolate(flat) aspect_f_len: vec2f,
    @location(4) @interpolate(flat) day_night_alpha: f32,
    @location(5) @interpolate(flat) globe_h: f32,
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
        if(camera.scale > GLOBE_SCALE) {
            coord = camera.globe_view_proj * transform_to_globe_position(pos.position.xy);
        } else {
            coord = camera.view_proj * coord;
        }
    } else {
        coord.x *= camera.inv_screen_size.x;
        coord.x = 2.0*(coord.x - 0.5);
        coord.y *= camera.inv_screen_size.y;
        coord.y = 2.0*(coord.y - 0.5) * -1.0;
    }
    out.clip_position = vec4<f32>(ratio_fixed_modelpos.xyz, 0.0) + vec4(coord.xyz/coord.w, 1.0);
    out.color = vec4f(model.color.rgb, model.color.a * pos.color_alpha);
    out.uv = model.uv;
    if(camera.scale > GLOBE_SCALE) {
        let unprojected = camera.inv_proj * out.clip_position;
        out.uv_cube = (camera.inv_globe_view * unprojected).xyz;
        let f_len = abs(camera.proj[1][1]);
        out.aspect_f_len = vec2f(camera.inv_screen_size.y / camera.inv_screen_size.x, f_len);
        out.globe_h = length(camera.camera_pos_in_globe.xyz);
        out.day_night_alpha = smoothstep(7.0, 10.0, out.globe_h * f_len);
    }

    return out;    
}

@if(GENERAL_RGBA || GENERAL_R_FLOAT)
@group(1) @binding(0)
var t_tex: texture_2d<f32>;
@if(DEPTH)
@group(1) @binding(0)
var t_tex: texture_depth_2d;
@if(CUBE)
@group(1) @binding(0)
var t_tex: texture_cube<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(1) @binding(2)
var s_compare: sampler_comparison;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

@fragment
fn fs_main_globe(in: VertexOutput) -> @location(0) vec4<f32> {
    let ndc = in.uv * 2.0 - 1.0;
    let d = length(ndc * camera.inv_ndc_globe_r);
    if(d <= 1.0) {
        return vec4f(0.957, 0.953, 0.941, 1.0);
    }
    @if(CUBE)
    return textureSample(t_tex, s_diffuse, in.uv_cube);
    @else
    discard;
}

/// Based on https://iquilezles.org/articles/spherefunctions/
fn sphIntersect(ro: vec3<f32>, rd: vec3<f32>, sph: vec4<f32>) -> f32 {
    let oc = ro - sph.xyz;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - sph.w * sph.w;
    let h = b * b - c;

    if (h < 0.0) {
        return -1.0;
    }

    return -b - sqrt(h);
}

fn sphNormal(pos: vec3<f32>, sph: vec4<f32>) -> vec3<f32> {
    return (pos - sph.xyz) / sph.w;
}

const SUN_DIR = normalize(vec3(1.0, 0.3, -0.7));

@fragment
fn fs_main_globe_glow(in: VertexOutput) -> @location(0) vec4<f32> {
    let ndc = in.uv * 2.0 - 1.0;

    let d = length(ndc * camera.inv_ndc_globe_r);
    if(d > 1.1) {
        discard;
    }
    var koef = 1.0;
    if (d <= 1.0) {
        let ro = vec3(0.0, 0.0, -in.globe_h);
        let rd = normalize(vec3((ndc * vec2f(in.aspect_f_len.x, 1.0)), in.aspect_f_len.y));
        let sph = vec4(0.0, 0.0, 0.0, 1.0);
        let t = sphIntersect(ro, rd, sph);
        if(t > 0.0) {
            let hitPos = ro + t * rd;
            let normal = sphNormal(hitPos, sph);
            let diffuse = max(dot(normal, SUN_DIR), 0.0);
            koef = (diffuse + 0.1);
        }
    }

    let glow_color = vec4f(0.35, 0.60, 1.00, 1.0);
    if d <= 1.0 {
        let factor = (max(0.9, d) - 0.9) / 0.1;
        let final_factor = factor * factor;
        let dark_koef = 1.0 - koef;
        let light_koef = (1.0 - dark_koef);
        let blendedColor = mix(vec3f(1.0), glow_color.rgb, final_factor);
        let final_rgb = blendedColor * 1.5 * light_koef;
        let final_alpha = max(dark_koef * in.day_night_alpha, final_factor * light_koef);

        return vec4f(final_rgb, final_alpha);
    }

    let screen_normal = normalize(ndc * camera.inv_ndc_globe_r);
    let light_dir_2d = normalize(SUN_DIR.xy);
    let atmospheric_diffuse = max(dot(screen_normal, light_dir_2d), 0.0);
    let outer_light_koef = atmospheric_diffuse + 0.2;
    let glow = abs(1.0 - smoothstep(1.0, 1.1, d));
    let outer_rgb = glow_color.rgb * outer_light_koef;
    let outer_alpha = 0.85 * glow * clamp(outer_light_koef * 3.5, 0.0, 1.0);

    return vec4f(outer_rgb, outer_alpha);
}

const tex_border_x: f32 = 0.01;

@fragment
fn fs_main_textured(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let tex_size = textureDimensions(t_tex);

    let tex_border_y = (tex_border_x * (f32(tex_size.x) / f32(tex_size.y)));
    if in.uv.x <= tex_border_x || in.uv.x >= 1.0 - tex_border_x || in.uv.y <= tex_border_y || in.uv.y >= 1.0 - tex_border_y {
         return vec4(1.0, 0.0, 0.0, 1.0);
    }

    @if(GENERAL_RGBA) {
        return textureSample(t_tex, s_diffuse, in.uv.xy);
    }
    @if(GENERAL_R_FLOAT) {
        let sample = textureSample(t_tex, s_diffuse, in.uv.xy);
        return vec4f(vec3f(1.0 - sample.r), 1.0);
    }
    @if(DEPTH) {
        let sample = textureSample(t_tex, s_diffuse, in.uv.xy);
        return vec4f(vec3f(sample), 1.0);
    }
    @if(CUBE) {
        return vec4f(1.0);
    }
}

@fragment
fn fs_main_tex_storage(in: VertexOutput) -> @location(0) vec4<f32> {
    let a_koef = max(0.0, 1.0 - camera.scale * 0.5);
    if a_koef <= 0.0 {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    var result = 0.0;
    let dims = vec2f(textureDimensions(t_tex));
    let texelSize = 1.0 / dims;
    let base = (floor(in.uv * dims) + 0.5) * texelSize;

    for (var y = -1; y <= 1; y += 2) {
        for (var x = -1; x <= 1; x += 2) {
            let offset = base + (vec2f(f32(x), f32(y)) - 0.5) * texelSize;
            @if(GENERAL_R_FLOAT)
            result += textureSample(t_tex, s_diffuse, offset).r;
        }
    }
    result = result / 4.0;

    return vec4f(0.0, 0.0, 0.0, result * a_koef);
}