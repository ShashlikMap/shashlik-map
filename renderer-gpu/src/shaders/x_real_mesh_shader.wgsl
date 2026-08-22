enable wgpu_mesh_shader;

const VERTEX_COUNT_1 = 9u;
const VERTEX_COUNT_2 = 6u;
const PRIM_COUNT_1 = VERTEX_COUNT_1 - 2u;
const PRIM_COUNT_2 = VERTEX_COUNT_2 - 2u;
const VERTEX_COUNT = VERTEX_COUNT_1 + VERTEX_COUNT_2;
const PRIMITIVE_COUNT = PRIM_COUNT_1 + PRIM_COUNT_2;

const positions = array(
    vec4( 0.0000,  0.8000, 0., 1.),
    vec4(-0.5142,  0.6128, 0., 1.),
    vec4(-0.7878,  0.1389, 0., 1.),
    vec4(-0.6928, -0.4000, 0., 1.),
    vec4(-0.2736, -0.7518, 0., 1.),
    vec4( 0.2736, -0.7518, 0., 1.),
    vec4( 0.6928, -0.4000, 0., 1.),
    vec4( 0.7878,  0.1389, 0., 1.),
    vec4( 0.5142,  0.6128, 0., 1.),

    // second one
    vec4( 0.0000,  0.4000, 0., 1.),
    vec4(-0.3464,  0.2000, 0., 1.),
    vec4(-0.3464, -0.2000, 0., 1.),
    vec4( 0.0000, -0.4000, 0., 1.),
    vec4( 0.3464, -0.2000, 0., 1.),
    vec4( 0.3464,  0.2000, 0., 1.),
);

const SOME_COLOR1 = vec4(0.7, 0.0, 0.0, 1.0);
const SOME_COLOR2 = vec4(0.7, 0.7, 0.0, 1.0);

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct PrimitiveOutput {
    @builtin(triangle_indices) indices: vec3<u32>,
}

struct MeshOutput {
    @builtin(vertices) vertices: array<VertexOutput, VERTEX_COUNT>,
    @builtin(primitives) primitives: array<PrimitiveOutput, PRIMITIVE_COUNT>,
    @builtin(vertex_count) vertex_count: u32,
    @builtin(primitive_count) primitive_count: u32,
}

var<workgroup> mesh_output: MeshOutput;

@mesh(mesh_output)
@workgroup_size(64)
fn ms_main(@builtin(local_invocation_id) thread_id: vec3<u32>) {
    if thread_id.x == 0 {
        mesh_output.vertex_count = VERTEX_COUNT;
        mesh_output.primitive_count = PRIMITIVE_COUNT;
    }

    workgroupBarrier();

    if thread_id.x < VERTEX_COUNT {
        mesh_output.vertices[thread_id.x].position = positions[thread_id.x];
        let color = select(SOME_COLOR1, SOME_COLOR2, thread_id.x >= VERTEX_COUNT_1);
        mesh_output.vertices[thread_id.x].color = color;
    }

    if thread_id.x < PRIMITIVE_COUNT {
        let is_second_fan = thread_id.x >= PRIM_COUNT_1;
        let base_vertex = select(0u, VERTEX_COUNT_1, is_second_fan);
        let local_prim = select(thread_id.x, thread_id.x - PRIM_COUNT_1, is_second_fan);

        mesh_output.primitives[thread_id.x].indices = vec3<u32>(
            base_vertex,
            base_vertex + local_prim + 1u,
            base_vertex + local_prim + 2u
        );
    }
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color;
}
