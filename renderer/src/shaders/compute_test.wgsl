@group(0) @binding(0)
var<storage, read_write> styles: vec4f;

@compute @workgroup_size(64)
fn compute_main(
) {
    styles.y = 0.0;
}