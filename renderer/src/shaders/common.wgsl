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

fn pcf(t_depth: texture_depth_2d, s_compare: sampler_comparison, coord: vec2f, texelSize: vec2f, bias: f32) -> f32 {
    var shadow = 0.0;
    shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(-1.0, -1.0) * texelSize, bias));
    shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(1.0, 1.0) * texelSize, bias));
    shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(-1.0, 1.0) * texelSize, bias));
    shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(1.0, -1.0) * texelSize, bias));
    if(shadow == 0.0 || shadow == 4.0) {
        shadow *= 0.25;
    } else {
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(-1.0, 0.0) * texelSize, bias));
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(0.0, -1.0) * texelSize, bias));
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(0.0, 0.0) * texelSize, bias));
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(0.0, 1.0) * texelSize, bias));
        shadow += (textureSampleCompare(t_depth, s_compare, coord + vec2f(1.0, 0.0) * texelSize, bias));
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