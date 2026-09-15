
// TODO this has to come from Rust, maybe push constants?
const GLOBE_SCALE: f32 = 12000.0;

const PI: f32 = 3.14159265359;
const MAP_SIZE: f32 = 16777216.0;
const GLOBE_R: f32 = MAP_SIZE / (2.0 * PI);

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