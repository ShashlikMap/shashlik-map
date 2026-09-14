import super::common::PI;
import super::common::MAP_SIZE;
import super::common::GLOBE_R;

fn transform_to_globe_position(position: vec2f) -> vec4f {
    let lat = 2.0 * atan(exp(PI * (1.0 - 2.0 * (position.y / MAP_SIZE)))) - (PI * 0.5);
    let lon = 2.0 * PI * ((position.x / MAP_SIZE) - 0.5);
    let globe = vec3<f32>(
        cos(lat) * sin(lon),
        cos(lat) * cos(lon),
        sin(lat),
    ) * GLOBE_R;
    return vec4<f32>(globe, 1.0);
}