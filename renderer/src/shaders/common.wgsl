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
    let texelSize = 1.0 / tex_size;

    let uv = coord * tex_size;
    var base_uv = floor(uv + 0.5);
    let s = (uv.x + 0.5 - base_uv.x);
    let t = (uv.y + 0.5 - base_uv.y);
    base_uv -= 0.5;
    base_uv *= texelSize;

    let uw0 = (4.0 - 3.0 * s);
    let uw1 = 7.0;
    let uw2 = (1.0 + 3.0 * s);

    let u0 = (3.0 - 2.0 * s) / uw0 - 2.0;
    let u1 = (3.0 + s) / uw1;
    let u2 = s / uw2 + 2.0;

    let vw0 = (4.0 - 3.0 * t);
    let vw1 = 7.0;
    let vw2 = (1.0 + 3.0 * t);

    let v0 = (3.0 - 2.0 * t) / vw0 - 2.0;
    let v1 = (3.0 + t) / vw1;
    let v2 = t / vw2 + 2.0;

    var sum = 0.0;

    sum += uw0 * vw0 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u0, v0) * texelSize), depth_with_bias);
    sum += uw1 * vw0 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u1, v0) * texelSize), depth_with_bias);
    sum += uw2 * vw0 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u2, v0) * texelSize), depth_with_bias);

    sum += uw0 * vw1 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u0, v1) * texelSize), depth_with_bias);
    sum += uw1 * vw1 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u1, v1) * texelSize), depth_with_bias);
    sum += uw2 * vw1 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u2, v1) * texelSize), depth_with_bias);

    sum += uw0 * vw2 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u0, v2) * texelSize), depth_with_bias);
    sum += uw1 * vw2 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u1, v2) * texelSize), depth_with_bias);
    sum += uw2 * vw2 * textureSampleCompare(t_depth, s_compare, base_uv + (vec2(u2, v2) * texelSize), depth_with_bias);

    return sum * (1.0 / 144.0);
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

fn frag_pos_from_ray(camera: CameraUniform, uv: vec2f) -> vec3f {
    let near_world1 = camera.view_proj_inv * vec4f(uv.xy, 0.0, 1.0);
    let near_world = near_world1.xyz / near_world1.w;
    let far_world1 = camera.view_proj_inv * vec4f(uv.xy, 1.0, 1.0);
    let far_world = far_world1.xyz / far_world1.w;

    var u = -near_world.z / (far_world.z - near_world.z);
    if u < 0.0 {
        u = 1.0 - u;
    }
    return near_world + u * (far_world - near_world);
}