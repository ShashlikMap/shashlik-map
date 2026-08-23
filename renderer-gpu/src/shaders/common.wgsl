struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    view_proj_inv: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    view_tr_inv: mat4x4<f32>,
    inv_screen_size: vec2<f32>,
    scale: f32,
    p2_scale: f32,
    scale_2d_3d: f32
};

fn shadow_map(t_depth: texture_depth_2d, s_compare: sampler_comparison, coord: vec2f, blur_size: f32, depth_with_bias: f32) -> f32 {
    @if(CASTANO)
    return castano(t_depth, s_compare, coord, depth_with_bias);

    return pcf(t_depth, s_compare, coord, blur_size, depth_with_bias);
}

// https://github.com/bevyengine/bevy/blob/bafb203232178ac26b596a1ea53e8f65a2d7e0d8/crates/bevy_pbr/src/render/shadow_sampling.wgsl#L105
// https://www.ludicon.com/castano/blog/articles/shadow-mapping-summary-part-1/
@if(CASTANO)
fn castano(t_depth: texture_depth_2d, s_compare: sampler_comparison, coord: vec2f, depth_with_bias: f32) -> f32 {
    let tex_size = vec2f(textureDimensions(t_depth));
    let texel_size = 1.0 / tex_size;

    let uv = coord * tex_size;
    let base_uv_pixels = floor(uv + 0.5);

    let st = uv + 0.5 - base_uv_pixels;
    let s = st.x;
    let t = st.y;
    let base_uv = (base_uv_pixels - 0.5) * texel_size;

    let uw = vec3f(4.0 - 3.0 * s, 7.0, 1.0 + 3.0 * s);
    let vw = vec3f(4.0 - 3.0 * t, 7.0, 1.0 + 3.0 * t);

    let inv_uw = 1.0 / uw;
    let inv_vw = 1.0 / vw;

    let u_offsets = vec3f(3.0 - 2.0 * s, 3.0 + s, s) * inv_uw + vec3f(-2.0, 0.0, 2.0);
    let v_offsets = vec3f(3.0 - 2.0 * t, 3.0 + t, t) * inv_vw + vec3f(-2.0, 0.0, 2.0);

    let u_tex = u_offsets * texel_size.x;
    let v_tex = v_offsets * texel_size.y;

    let c_bl = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.x, v_tex.x), depth_with_bias); // Bottom-Left
    let c_br = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.z, v_tex.x), depth_with_bias); // Bottom-Right
    let c_tl = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.x, v_tex.z), depth_with_bias); // Top-Left
    let c_tr = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.z, v_tex.z), depth_with_bias); // Top-Right

    let corner_sum = c_bl + c_br + c_tl + c_tr;

    // Out if fully unshadowed (4.0) or fully shadowed (0.0)
    if (corner_sum == 4.0) {
        return 1.0;
    }
    if (corner_sum == 0.0) {
        return 0.0;
    }

    let c_bm = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.y, v_tex.x), depth_with_bias); // Bottom-Middle

    let c_ml = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.x, v_tex.y), depth_with_bias); // Middle-Left
    let c_mm = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.y, v_tex.y), depth_with_bias); // Middle-Middle
    let c_mr = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.z, v_tex.y), depth_with_bias); // Middle-Right

    let c_tm = textureSampleCompare(t_depth, s_compare, base_uv + vec2f(u_tex.y, v_tex.z), depth_with_bias); // Top-Middle

    var sum = 0.0;
    sum += uw.x * vw.x * c_bl;
    sum += uw.y * vw.x * c_bm;
    sum += uw.z * vw.x * c_br;

    sum += uw.x * vw.y * c_ml;
    sum += uw.y * vw.y * c_mm;
    sum += uw.z * vw.y * c_mr;

    sum += uw.x * vw.z * c_tl;
    sum += uw.y * vw.z * c_tm;
    sum += uw.z * vw.z * c_tr;

    // 1.0 / 144.0
    return sum * 0.006944444;
}

fn pcf(t_depth: texture_depth_2d, s_compare: sampler_comparison, coord: vec2f, blur_size: f32, depth_with_bias: f32) -> f32 {
    let texelSize = max(blur_size, 1.0) / vec2f(textureDimensions(t_depth));
    var shadow = 0.0;
    shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(-1.0, -1.0) * texelSize, depth_with_bias));
    shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(1.0, 1.0) * texelSize, depth_with_bias));
    shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(-1.0, 1.0) * texelSize, depth_with_bias));
    shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(1.0, -1.0) * texelSize, depth_with_bias));
    if(shadow == 0.0 || shadow == 4.0) {
        shadow *= 0.25;
    } else {
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(-1.0, 0.0) * texelSize, depth_with_bias));
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(0.0, -1.0) * texelSize, depth_with_bias));
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(0.0, 0.0) * texelSize, depth_with_bias));
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(0.0, 1.0) * texelSize, depth_with_bias));
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(1.0, 0.0) * texelSize, depth_with_bias));
        shadow /= 9.0;
    }
    return shadow;
}