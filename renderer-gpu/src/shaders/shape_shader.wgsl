import super::common::CameraUniform;

// Vertex shader
const PARAMS_COUNT : i32 = 12; // 12 is mat4x3!

struct StyleUniform {
    params: mat4x3<f32>
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// there is a chance that dyn array without size might not be working on every platform
@group(1) @binding(0)
var<storage, read> styles: array<StyleUniform>;

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
    @location(11) bbox: vec4<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) style1: vec3<f32>,
    @location(1) @interpolate(flat) style2: vec3<f32>,
    @location(2) @interpolate(flat) style3: vec3<f32>,
    @location(3) @interpolate(flat) style4: vec3<f32>,
    @location(4) @interpolate(flat) outline_flag: u32,
    @location(5) color_alpha: f32,
    @location(6) vertex_pos_xy: vec2<f32>,
    @location(7) bbox: vec4<f32>,
    @location(8) uv_dist: vec3<f32>,
}

// TODO pass as a parameter
const inflate_factor: f32 = 0.24;

fn style_array_to_mat(out: ptr<function,VertexOutput>, params: mat4x3<f32>) {
    (*out).style1 = params[0];
    (*out).style2 = params[1];
    (*out).style3 = params[2];
    (*out).style4 = params[3];
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
    var out: VertexOutput;
    let model_position = model_matrix * vec4(model.position.xy, 0.0, 1.0);
    var modelpos = model_position.xyz + pos.position;

    style_array_to_mat(&out, styles[model.style_index].params);
    out.outline_flag = model.instance_index % 2;
    out.color_alpha = pos.color_alpha;

    // only two components for normal
    var normal_scale = vec3f(0.0, 0.0, 0.0);
    if(out.outline_flag == 0) {
        normal_scale = vec3(model.normal.xy * inflate_factor, 0.0);
    }

    let pointPos = modelpos.xyz + normal_scale.xyz;

    out.vertex_pos_xy = pointPos.xy;
    out.bbox = pos.bbox;
    // divide distance to scale, so dash shader works properly
    out.uv_dist = vec3f(model.uv, f32(model.dist) / camera.p2_scale);
    out.clip_position = camera.view_proj * vec4<f32>(pointPos, 1.0);
    return out;
}

// TODO pass as a parameter
const route_inflate_factor: f32 = 1.3;
@vertex
fn vs_main_route(
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

    style_array_to_mat(&out, styles[model.style_index].params);
    out.outline_flag = 1;
    if(with_normal) {
        out.outline_flag = model.instance_index % 2;
    }

    var pointPos = modelpos.xyz;
    if(with_normal) {
        var normal_scale = max(camera_scale, 0.75) * 0.5;
        if(model.instance_index % 2 == 0) {
            if(normal_scale < 0.0) {
                normal_scale /= route_inflate_factor;
            } else {
                normal_scale *= route_inflate_factor;
            }
        }
        pointPos += normalize(vec3(model.normal, 0.0)) * normal_scale;
    }

    out.vertex_pos_xy = pointPos.xy;
    out.uv_dist = vec3f(model.uv, f32(model.dist));
    out.clip_position = camera.view_proj * vec4<f32>(pointPos, 1.0);
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

    style_array_to_mat(&out, styles[model.style_index].params);
    // FIXME Disable outlining for screen shapes for a while
    out.outline_flag = 1; //model.instance_index % 2;
    out.color_alpha = pos.color_alpha;

    var pointPos = ratio_fixed_modelpos.xyz;

    let coord = camera.view_proj * vec4<f32>(pos.position.xy, 0.0, 1.0);

    out.clip_position = vec4(pointPos, 0.0) + vec4(coord.xyz/coord.w, 1.0);

    return out;
}

//0 - matrix[0][0]
//1 - matrix[0][1]
//2 - matrix[0][2]
//3 - matrix[1][0]
//4 - matrix[1][1]
//5 - matrix[1][2]
//6 - matrix[2][0]
//7 - matrix[2][1]
//8 - matrix[2][2]
//9 - matrix[3][0]
//10 - matrix[3][1]
//11 - matrix[3][2]

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // ignore if both are zero
    if in.bbox.z > 0.0 || in.bbox.w > 0.0 {
        if in.vertex_pos_xy.x < in.bbox.x || in.vertex_pos_xy.x > in.bbox.x + in.bbox.z {
            discard;
        }
        if in.vertex_pos_xy.y < in.bbox.y || in.vertex_pos_xy.y > in.bbox.y + in.bbox.w {
            discard;
        }
    }
    let style = mat4x3<f32>(
            in.style1,
            in.style2,
            in.style3,
            in.style4,
        );
    // FIXME Requires better solution for param type
    let style_type = u32(style[0][0]);

    var res_color = vec4(0.0, 0.0, 0.0, 1.0);
    if(style_type == 0) {
        res_color = solid_style(in.outline_flag, style);
    } else if(style_type == 1) {
        res_color = border_style(in.outline_flag, style);
    } else if(style_type == 2) {
        res_color = dashed_style(in.outline_flag, in.uv_dist, style);
    } else {
        res_color = vec4(0.0, 0.0, 0.0, 1.0);
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

fn solid_style(outline_flag: u32, params: mat4x3<f32>) -> vec4<f32> {
    if(outline_flag == 0) {
        discard;
    }
    let fill_color = vec4(params[0][1], params[0][2], params[1][0], params[1][1]);
    return fill_color;
}

fn border_style(outline_flag: u32, params: mat4x3<f32>) -> vec4<f32> {
    let fill_color = vec4(params[0][1], params[0][2], params[1][0], params[1][1]);
    if(outline_flag == 0) {
        let koef = params[1][2];
        return vec4(fill_color.x * koef, fill_color.y * koef, fill_color.z * koef, 1.0);
    }
    return fill_color;
}

fn dashed_style(outline_flag: u32, uv_dist: vec3f, params: mat4x3<f32>) -> vec4<f32> {
    let dash_style = u32(params[3][0]); // 0: solid, 1: circle
    if(outline_flag == 0) {
        // TODO Border + Dashed later
        discard;
    }

    let fill_color = vec4(params[0][1], params[0][2], params[1][0], params[1][1]);
    let dash_color = vec4(params[1][2], params[2][0], params[2][1], params[2][2]);

    if(dash_style == 1) {
        let cirlce_alpha0 = circle(uv_dist.xy, 0.85);
        let cirlce_alpha1 = circle(uv_dist.xy, 0.45);
        return mix(vec4(fill_color.rgb, cirlce_alpha0), vec4(dash_color.rgb, cirlce_alpha1), cirlce_alpha1);
    } else {
        // uv_dist.z - is a distance
        return dash_solid(uv_dist.z, dash_color, fill_color);
    }
}

const freq = 0.5; // the less the longer dashes
fn dash_solid(dist: f32, extra_color: vec4f, main_color: vec4f) -> vec4f {
    let dash = step(0.5, fract(dist * freq));

    if(dash <= 0.0) {
        return extra_color;
    } else {
        return main_color;
    }
}