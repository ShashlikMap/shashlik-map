import super::common::CameraUniform;
import super::shape_styles;
import super::shape_styles::{ShapeStyle, ShapeSubStyle};
import super::globe_common::GLOBE_SCALE;
import super::globe_common::transform_to_globe_position;

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> styles: array<vec4f>;

@group(2) @binding(0)
var<storage, read> indirect_instances: array<InstanceInput>;

@group(2) @binding(1)
var<storage, read> culled: array<u32>;

struct VertexInput {
    @builtin(instance_index) instance_index : u32,
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) dist: u32,
    @location(4) style_index: u32,
}

struct InstanceInput {
    @location(5) position: vec3<f32>,
    @location(6) color_alpha: f32,
    @location(7) model_matrix_0: vec4<f32>,
    @location(8) model_matrix_1: vec4<f32>,
    @location(9) model_matrix_2: vec4<f32>,
    @location(10) model_matrix_3: vec4<f32>,
    @location(11) bbox: vec4<f32>,
    @location(12) normal_scale: f32
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) style_type_subtype: vec2<u32>,
    @location(1) @interpolate(flat) style_color_1: vec4<f32>,
    @location(2) @interpolate(flat) style_color_2: vec4<f32>,
    @location(3) color_alpha: f32,
    @location(4) vertex_pos_xy: vec2<f32>,
    @location(5) bbox: vec4<f32>,
    @location(6) uv_dist: vec3<f32>,
}

// TODO pass as a parameter
const inflate_factor: f32 = 0.24;

fn fill_styles(out: ptr<function,VertexOutput>, style_index: u32, scale: f32, outline_flag: u32) {
    let header = styles[style_index];
    let style_type = u32(header[0]);
    let fill_color = styles[style_index + 1];

    (*out).style_type_subtype = vec2(style_type, shape_styles::SUB_STYLE_SOLID);
    (*out).style_color_1 = fill_color;

    @if(OUTLINE_DEBUG)
    if(outline_flag == 0) {
        (*out).style_type_subtype = vec2(0u, shape_styles::SUB_STYLE_SOLID);
        (*out).style_color_1 = vec4f(1.0, 0.0, 0.0, 1.0);
        return;
    }

    switch style_type {
        case shape_styles::STYLE_BORDER: {
            if(outline_flag == 0) {
                let border_koef = header[1];
                let border_color = vec4(fill_color.xyz * border_koef, 1.0 / max(1.0, scale));
                (*out).style_color_1 = border_color;
            }
        }
        case shape_styles::STYLE_DASH: {
            (*out).style_type_subtype.y = u32(header[1]); // 0: solid, 1: circle, 2: tdash
            (*out).style_color_2 = styles[style_index + 2];
        }
        default : {}
    }
}

fn handle_flat_globe(out: ptr<function, VertexOutput>, position: vec3f) {
    if(camera.scale > GLOBE_SCALE) {
        // drop bbox, so it won't be checked in FS
        (*out).bbox.z = 0.0;
        (*out).bbox.w = 0.0;
        (*out).clip_position = camera.globe_view_proj * transform_to_globe_position(position.xy);
    } else  {
        (*out).clip_position = camera.view_proj * vec4<f32>(position, 1.0);
    }
}

@vertex
fn vs_main(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    if(pos.color_alpha <= 0.0) {
        // degrading polygon to drop FS stage at all
        return VertexOutput();
    }
    let model_matrix = mat4x4<f32>(
            pos.model_matrix_0,
            pos.model_matrix_1,
            pos.model_matrix_2,
            pos.model_matrix_3,
    );
    var out: VertexOutput;
    let model_position = model_matrix * vec4(model.position.xy, 0.0, 1.0);
    var modelpos = model_position.xyz + pos.position;

    let outline_flag = model.instance_index % 2;
    fill_styles(&out, model.style_index, camera.scale, outline_flag);
    out.color_alpha = pos.color_alpha;

    // only two components for normal
    let factor = select(0.0, max(1.0, camera.scale * 0.5), outline_flag == 0u); // increase border with scale
    let normal_scale = vec3(model.normal.xy * inflate_factor * factor, 0.0);

    let pointPos = modelpos.xyz + normal_scale.xyz + vec3(model.normal * (pos.normal_scale), 0.0);

    out.vertex_pos_xy = pointPos.xy;
    out.bbox = pos.bbox;
    // divide distance to scale, so dash shader works properly
    out.uv_dist = vec3f(model.uv, f32(model.dist) / camera.p2_scale);

    handle_flat_globe(&out, pointPos);

    return out;
}

// TODO pass as a parameter
const indirect_inflate_factor: f32 = 1.3;
@vertex
fn vs_main_indirect(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let with_normal = model.normal.x != 0.0 || model.normal.y != 0.0;

    let camera_scale = max(camera.scale, 0.25);

    out.color_alpha = 1.0;

    var instance_index = model.instance_index;
    if(!with_normal) {
        instance_index = culled[model.instance_index];
    }

    if(!with_normal && camera_scale >= 0.0) {
        out.color_alpha = indirect_instances[instance_index].color_alpha;
    }

    var model_position = vec4(model.position.xy, 0.0, 1.0);

    if(!with_normal) {
        let scale_m = mat4x4(camera_scale, 0.0, 0.0, 0.0, 0.0, camera_scale, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        model_position = scale_m * model_position;
    }

    var modelpos = model_position.xyz + indirect_instances[instance_index].position;

    let outline_flag = select(1, model.instance_index % 2, with_normal);
    fill_styles(&out, model.style_index, 1.0, outline_flag);

    var pointPos = modelpos.xyz;
    if(with_normal) {
        var inflate_scale = 1.0;
        if(model.instance_index % 2 == 0) {
            inflate_scale *= indirect_inflate_factor;
        }
        let normal_scale = indirect_instances[instance_index].normal_scale;
        pointPos += normalize(vec3(model.normal, 0.0)) * normal_scale * 0.5 * inflate_scale;
    }

    out.vertex_pos_xy = pointPos.xy;
    out.uv_dist = vec3f(model.uv, f32(model.dist));

    handle_flat_globe(&out, pointPos);

    return out;
}

@vertex
fn vs_main_screen(
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

    let model_position = model_matrix * vec4(model.position.xy, 0.0, 1.0);
    let ratio_fixed_modelpos = vec4(model_position.xy * vec2(2.0*camera.inv_screen_size.x, 2.0*camera.inv_screen_size.y), model_position.z, 1.0);

    // FIXME Disable outlining for screen shapes for a while
    fill_styles(&out, model.style_index, 0.0, 1);
    out.color_alpha = pos.color_alpha;

    var pointPos = ratio_fixed_modelpos.xyz;

    // TODO We may need also use handle_flat_globe here, but there are no cases to verify it

    let coord = camera.view_proj * vec4<f32>(pos.position.xy, 0.0, 1.0);

    out.clip_position = vec4(pointPos, 0.0) + vec4(coord.xyz/coord.w, 1.0);

    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // ignore if both are zero
    let has_bounds = in.bbox.z > 0.0 || in.bbox.w > 0.0;
    let outside_x = in.vertex_pos_xy.x < in.bbox.x || in.vertex_pos_xy.x > in.bbox.x + in.bbox.z;
    let outside_y = in.vertex_pos_xy.y < in.bbox.y || in.vertex_pos_xy.y > in.bbox.y + in.bbox.w;
    if has_bounds && (outside_x || outside_y) {
        discard;
    }

    let style_type = in.style_type_subtype.x;

    var res_color = vec4(0.0, 0.0, 0.0, 1.0);
    switch style_type {
        case shape_styles::STYLE_SOLID, shape_styles::STYLE_BORDER: {
            res_color = in.style_color_1;
        }
        case shape_styles::STYLE_DASH: {
            res_color = dashed_style(in.uv_dist, in.style_color_1, in.style_color_2, in.style_type_subtype.y);
        }
        default : {
            res_color = vec4(0.0, 0.0, 0.0, 1.0);
        }
    }

    res_color.a *= in.color_alpha;

    return res_color;
}

fn circle(st: vec2f, radius: f32) -> f32 {
    let dist = vec2f(st.x - 0.5, st.y - 0.5);
	return 1.0 - smoothstep(radius-(radius*0.04),
                         radius+(radius*0.04),
                         dot(dist,dist)*4.0);
}

// TODO pass as a parameter?
const T_DASH_FACTOR: f32 = 30.0;
fn dashed_style(uv_dist: vec3f, color1: vec4f, color2: vec4f, dash_style: u32) -> vec4<f32> {
    let fill_color = color1;
    let dash_color = color2;

    switch dash_style {
        case default, shape_styles::SUB_STYLE_SOLID: {
            // uv_dist.z - is a distance
            return dash_solid(camera.p2_scale, uv_dist.z, dash_color, fill_color);
        }
        case shape_styles::SUB_STYLE_CIRCLE: {
            let cirlce_alpha0 = circle(uv_dist.xy, 0.85);
            let cirlce_alpha1 = circle(uv_dist.xy, 0.45);
            return mix(vec4(fill_color.rgb, cirlce_alpha0), vec4(dash_color.rgb, cirlce_alpha1), cirlce_alpha1);
        }
        case shape_styles::SUB_STYLE_TDASH: {
            // uv_dist.x - is a side dist in 0.0..1.0 range
            // converted to -1.0..1.0 range
            let u = (uv_dist.x - 0.5) * 2.0;
            return dash_solid(camera.p2_scale, T_DASH_FACTOR * u, dash_color, fill_color);
        }
    }
}

const freq = 0.5; // the less the longer dashes
fn dash_solid(p2_scale: f32, dist: f32, extra_color: vec4f, main_color: vec4f) -> vec4f {
    // prevents dash to be too short when a line width longer than a default dash
    let freq_fixed = select(freq, freq * 0.2 * p2_scale, p2_scale <= 2.0);
    let dash = step(0.5, fract(dist * freq_fixed));
    return select(main_color, extra_color, dash <= 0.0);
}