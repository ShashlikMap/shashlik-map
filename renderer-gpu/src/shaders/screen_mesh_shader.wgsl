import super::common::CameraUniform;
import super::common::camera;
import super::globe_common::GLOBE_SCALE;
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

fn sphIntersect(ro: vec3<f32>, rd: vec3<f32>, sph: vec4<f32>) -> f32
{
    let oc: vec3<f32> = ro - sph.xyz;
    let b: f32 = dot(oc, rd);
    let c: f32 = dot(oc, oc) - sph.w * sph.w;
    let h: f32 = b * b - c;

    if (h < 0.0) {
        return -1.0;
    }

    let h_sqrt: f32 = sqrt(h);
    return -b - h_sqrt;
}

fn sphNormal(pos: vec3<f32>, sph: vec4<f32>) -> vec3<f32>
{
    return (pos - sph.xyz) / sph.w;
}

@fragment
fn fs_main_globe_glow(in: VertexOutput) -> @location(0) vec4<f32> {
    let ndc = in.uv * 2.0 - 1.0;

    let d = length(ndc * camera.inv_ndc_globe_r);
    if(d > 1.15) {
        discard;
    }
    var koef = 1.0;
    if (d <= 1.0) {
        let fov_radians = 0.4060005080818896;
        let ro = vec3(0.0, 0.0, -camera.rig);
        let focal_length: f32 = 1.0 / tan(fov_radians * 0.5);
        let rd = normalize(vec3((ndc * vec2f(camera.inv_screen_size.y / camera.inv_screen_size.x, 1.0)), focal_length));
        let r = d;//camera.globe_r / 0.70710678;
        let sph = vec4(0.0, 0.0, 0.0, 2670176.857720436);
        let t = sphIntersect(ro, rd, sph);
        var color = vec3(0.1, 0.15, 0.2);

        if(t > 0.0) {
            // 5. Calculate the exact 3D point where the ray hit the sphere
            let hitPos = ro + t * rd;

            // 6. Get the surface normal at that point
            let normal = sphNormal(hitPos, sph);

            // 7. Apply simple lighting (directional light coming from top-right)
            let lightDir = normalize(vec3(1.0, 1.0, -0.3));
            let diffuse = max(dot(normal, lightDir), 0.0);

            // Mix a base sphere color (e.g., orange) with the lighting
            koef = (diffuse + 0.3);
            color = vec3(1.0, 0.5, 0.2) * (diffuse + 0.1); // 0.1 is ambient light
//            return vec4f(1.0, 0.0, 0.0, 1.0);
//            return vec4f(hitPos, 1.0);

        }
//        return vec4f(color, 1.0);
    }
    let glow_color = vec4f(0.35, 0.60, 1.00, 1.0);
    let dark_koef = 1.0 - koef;
    if d <= 1.0 {
        let factor = (max(0.9, d) - 0.9) / 0.1;
        let final_factor = factor * factor;

        let light_koef = 1.0 - dark_koef; // 1.0 = sunlit day side, 0.0 = dark night side

        // 1. FIXED COLOR GRADIENT:
        // We blend from your base glow color (blue) toward pure white at the absolute outer edge.
        // By multiplying the white target by light_koef, only the sunlit side reaches that bright white highlight!
        let bright_white_peak = vec3f(1.0);// * light_koef;
        let blendedColor = mix(bright_white_peak, glow_color.rgb, final_factor);

        // 2. SUNLIT EDGE MULTIPLIER:
        // Ensure the entire colored atmosphere ring fades to black on the night side.
        let final_rgb = blendedColor * light_koef;

        // 3. ALPHA OPACITY MASK:
        // Keep the main body shadow (dark_koef) across the planet center,
        // and smoothly scale up the atmosphere visibility near the edges.
        let final_alpha = max(dark_koef, final_factor * light_koef);

        return vec4f(final_rgb, final_alpha);
    }

    let glow = abs(1.0 - smoothstep(1.0, 1.15, d));
    return vec4f(glow_color.rgb, 0.85 * glow);
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