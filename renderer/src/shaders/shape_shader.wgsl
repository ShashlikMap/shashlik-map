// Vertex shader
const PARAMS_COUNT : i32 = 12;

struct CameraUniform {
    view_proj: mat4x4<f32>,
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
    @location(2) style_index: u32,
}

struct InstanceInput {
    @location(3) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) style_index: u32,
    @location(1) outline_flag: u32,
}

// TODO pass as a parameter
const inflate_factor: f32 = 0.06;

@vertex
fn vs_main(
    model: VertexInput,
    pos: InstanceInput
) -> VertexOutput {
    var out: VertexOutput;
    var modelpos = model.position + pos.position;

    out.style_index = model.style_index;
    out.outline_flag = model.instance_index % 2;

    var pointPos = modelpos.xyz;
    if(model.instance_index % 2 == 0) {
        // only two components for normal
        pointPos += vec3(model.normal.xy * inflate_factor, 0.0);
    }

    out.clip_position = camera.view_proj * vec4<f32>(pointPos, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let style_params = styles[in.style_index].params;
    // FIXME Requires better solution for param type
    let style_type = u32(round(style_params[0]));

     if(style_type == 0) {
        return solid_style(in.outline_flag, style_params);
     } else if(style_type == 1) {
        return border_style(in.outline_flag, style_params);
     }

    return vec4(0.0, 0.0, 0.0, 1.0);
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