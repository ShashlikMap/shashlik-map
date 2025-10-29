use lyon::geom::point;
use lyon::lyon_tessellation::VertexBuffers;
use lyon::path::Path;
use renderer::draw_commands::MeshVertex;
use std::io::BufReader;
use tobj::LoadError;

pub struct MeshLoader {}

impl MeshLoader {
    pub fn load_simple_puck() -> Path {
        let mut builder = Path::builder();
        builder.begin(point(0.0, 0.0));
        builder.line_to(point(2.0, -5.0));
        builder.line_to(point(-2.0, -5.0));
        builder.end(true);
        let path = builder.build();
        path
    }

    pub fn load_test_polygon_path() -> Path {
        let mut builder = Path::builder();
        builder.begin(point(0.0, 0.0));
        builder.line_to(point(1.0, 0.0));
        builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
        builder.cubic_bezier_to(point(1.0, 1.0), point(0.0, 1.0), point(0.0, 0.0));
        builder.end(true);
        let path = builder.build();
        path
    }

    pub fn load_test_line_path() -> Path {
        let mut builder = Path::builder();
        builder.begin(point(0.0, 0.0));
        builder.line_to(point(1.0, 0.0));
        builder.line_to(point(2.5, 1.0));
        builder.line_to(point(-2.4, 0.7));
        builder.end(false);
        let path = builder.build();
        path
    }

    pub fn load_test_line2_path() -> Path {
        let mut builder = Path::builder();
        builder.begin(point(0.0, 2.0));
        builder.line_to(point(-1.0, -0.0));
        builder.line_to(point(0.5, 0.0));
        builder.line_to(point(-1.4, 1.7));
        builder.end(false);
        let path = builder.build();
        path
    }
    
    pub fn load_from_obj(data: &[u8]) -> VertexBuffers<MeshVertex, u32> {
        let opts = &tobj::LoadOptions {
            triangulate: true,
            single_index: false,
            ..Default::default()
        };

        let cube_obj = tobj::load_obj_buf(&mut BufReader::new(data), opts, |_mat_path| {
            Err(LoadError::GenericFailure)
        })
        .ok()
        .unwrap();
        let mut vertex_buffers = VertexBuffers::new();
        cube_obj.0.iter().for_each(|model| {
            let vertices_mesh = (0..model.mesh.positions.len() / 3)
                .map(|i| {
                    let (x, y, z) = (
                        model.mesh.positions[i * 3],
                        model.mesh.positions[i * 3 + 1],
                        model.mesh.positions[i * 3 + 2],
                    );
                    let (nx, ny, nz) = if model.mesh.normals.is_empty() {
                        (0.0, 0.0, 0.0)
                    } else {
                        if i * 3 < model.mesh.normals.len() {
                            (
                                model.mesh.normals[i * 3],
                                model.mesh.normals[i * 3 + 1],
                                model.mesh.normals[i * 3 + 2],
                            )
                        } else {
                            (0.0, 0.0, 0.0)
                        }
                    };
                    MeshVertex {
                        position: [x, y, z],
                        normals: [nx, ny, nz],
                    }
                })
                .collect::<Vec<_>>();

            vertex_buffers.vertices.extend(vertices_mesh);
            vertex_buffers.indices.extend(model.mesh.indices.clone());
        });

        vertex_buffers
    }
}
