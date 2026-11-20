import super::common::camera;
import super::globe_common::GLOBE_SCALE;
import super::globe_common::transform_to_globe_position;

// Keep the ground (roads, labels context) readable under the rain
const RAIN_OPACITY: f32 = 0.7;

@group(1) @binding(0)
var t_raster: texture_2d<f32>;
@group(1) @binding(1)
var s_raster: sampler;

struct VertexInput {
    // position inside the tile, [0..1], y goes down (south) like the world Y
    @location(0) uv: vec2<f32>,
}

struct InstanceInput {
    // tile top-left corner, relative to camera offset (zero offset in Globe view)
    @location(1) origin: vec2<f32>,
    @location(2) size: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // > 0 if the point is on the visible side of the globe
    @location(1) globe_visibility: f32,
}

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let position = instance.origin + model.uv * instance.size;
    if(camera.scale > GLOBE_SCALE) {
        let globe_position = transform_to_globe_position(position);
        out.clip_position = camera.globe_view_proj * globe_position;
        // camera_pos_in_globe is in globe R units
        out.globe_visibility = dot(normalize(globe_position.xyz), camera.camera_pos_in_globe.xyz) - 1.0;
    } else {
        out.clip_position = camera.view_proj * vec4<f32>(position, 0.0, 1.0);
        out.globe_visibility = 1.0;
    }
    out.uv = model.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // texture is premultiplied alpha to avoid dark fringes with linear filtering
    let color = textureSample(t_raster, s_raster, in.uv);
    if(in.globe_visibility < 0.0) {
        discard;
    }
    return color * RAIN_OPACITY;
}
