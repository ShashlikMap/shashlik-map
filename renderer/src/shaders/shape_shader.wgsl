// Vertex shader
const PARAMS_COUNT : i32 = 12;

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    ratio: f32,
};

struct StyleUniform {
    params: array<f32, PARAMS_COUNT>
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// there is a chance that dyn array without size might not be working on every platform
@group(1) @binding(0)
var<storage, read> styles: array<StyleUniform>;

struct VertexInput {
    @builtin(instance_index) instance_index : u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) dist: f32,
    @location(3) style_index: u32,
}

struct InstanceInput {
    @location(4) position: vec3<f32>,
    @location(5) color_alpha: f32,
    @location(6) model_matrix_0: vec4<f32>,
    @location(7) model_matrix_1: vec4<f32>,
    @location(8) model_matrix_2: vec4<f32>,
    @location(9) model_matrix_3: vec4<f32>,
    @location(10) bbox: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) style_index: u32,
    @location(1) outline_flag: u32,
    @location(2) color_alpha: f32,
    @location(3) vertex_pos_xy: vec2<f32>,
    @location(4) bbox: vec4<f32>,
    @location(5) dist: f32,
}

// TODO pass as a parameter
const inflate_factor: f32 = 0.06;

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
    let model_position = model_matrix * vec4(model.position.xyz, 1.0);
    var modelpos = model_position.xyz + pos.position;

    out.style_index = model.style_index;
    out.outline_flag = model.instance_index % 2;
    out.color_alpha = pos.color_alpha;

    var pointPos = modelpos.xyz;
    if(model.instance_index % 2 == 0) {
        // only two components for normal
        pointPos += vec3(model.normal.xy * inflate_factor, 0.0);
    }

    out.vertex_pos_xy = pointPos.xy;
    out.bbox = pos.bbox;
    out.dist = model.dist;
    out.clip_position = camera.view_proj * vec4<f32>(pointPos, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // ignore if both are zero
    if in.bbox.z > 0.0 || in.bbox.w > 0.0 {
        if in.vertex_pos_xy.x < in.bbox.x || in.vertex_pos_xy.x > in.bbox.x + in.bbox.z {
            discard;
        }
        // carefull with sings, they are different from X axis
        if in.vertex_pos_xy.y > in.bbox.y || in.vertex_pos_xy.y < in.bbox.y - in.bbox.w  {
            discard;
        }
    }
    let style_params = styles[in.style_index].params;
    // FIXME Requires better solution for param type
    let style_type = u32(round(style_params[0]));

    var res_color = vec4(0.0, 0.0, 0.0, 1.0);
    if(style_type == 0) {
        res_color = solid_style(in.outline_flag, style_params);
    } else if(style_type == 1) {
        res_color = border_style(in.outline_flag, style_params);
    } else if(style_type == 2) {
        res_color = dashed_style(in.outline_flag, in.dist, style_params);
    } else {
        res_color = vec4(0.0, 0.0, 0.0, 1.0);
    }

     res_color.a = in.color_alpha;

     return res_color;
}

fn solid_style(outline_flag: u32, params: array<f32, PARAMS_COUNT>) -> vec4<f32> {
    if(outline_flag == 0) {
        discard;
    }
    let fill_color = vec4(params[1], params[2], params[3], params[4]);
    return fill_color;
}

fn border_style(outline_flag: u32, params: array<f32, PARAMS_COUNT>) -> vec4<f32> {
    let fill_color = vec4(params[1], params[2], params[3], params[4]);
    if(outline_flag == 0) {
        let koef = params[5];
        return vec4(fill_color.x * koef, fill_color.y * koef, fill_color.z * koef, 1.0);
    }
    return fill_color;
}

fn dashed_style(outline_flag: u32, dist: f32, params: array<f32, PARAMS_COUNT>) -> vec4<f32> {
    let fill_color = vec4(params[1], params[2], params[3], params[4]);
    let dash_color = vec4(params[5], params[6], params[7], params[8]);
    if(outline_flag == 0) {
        // TODO Border + Dashed later
        discard;
    }
    
    return dash(4.0, 0.2*dist, dash_color, fill_color);
}

fn dash(freq: f32, dist: f32, extra_color: vec4f, main_color: vec4f) -> vec4f {
    let koef = 1.0 / freq;
    let lineUMod = (dist - (koef)*floor(dist/(koef))) * freq;
    let dash = smoothstep(koef, koef, length(lineUMod-0.5));

    var dashed_color = vec4(0.0, 0.0, 0.0, 0.0);

    if(dash <= 0.0) {
        dashed_color = extra_color;
    } else {
        dashed_color = main_color;
    }
    return dashed_color;
}