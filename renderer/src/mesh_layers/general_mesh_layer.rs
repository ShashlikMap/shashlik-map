use crate::mesh::mesh::Mesh;
use crate::mesh_layers::BaseMeshLayer;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::mesh_node::PositionedMesh;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::RenderPipeline;
use crate::vertex_attrs::GeneralInstanceInput;
use crate::GlobalContext;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};
use crate::nodes::SceneNode;

pub struct GeneralMeshLayer {
    mesh_pipeline: MeshPipeline,
    meshes: Vec<PositionedMesh<GeneralInstanceInput>>,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl GeneralMeshLayer {
    pub fn new(device: &Device) -> Self {
        GeneralMeshLayer {
            mesh_pipeline: MeshPipeline::new(device),
            meshes: vec![],
            pipeline: None,
        }
    }
    pub fn add(
        &mut self,
        device: &Device,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        mesh: Mesh,
    ) {
        self.meshes
            .push(mesh.to_positioned::<GeneralInstanceInput>(device, spatial_rx));
    }
}

impl BaseMeshLayer for GeneralMeshLayer {
    fn prepare(&mut self, device: &Device, config: &SurfaceConfiguration) {
        let descriptor = self.mesh_pipeline.prepare(device, config);
        self.pipeline = Some(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: descriptor.layout.as_ref(),
                vertex: wgpu::VertexState {
                    module: &descriptor.vertex.module,
                    entry_point: Some("vs_main"),
                    buffers: &*descriptor.vertex.buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &descriptor.vertex.module,
                    entry_point: Some("fs_main"),
                    targets: &*descriptor.fragment.unwrap().targets,
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: descriptor.primitive.cull_mode,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: descriptor.depth_stencil,
                multisample: descriptor.multisample,
                // Useful for optimizing shader compilation on Android
                cache: None,
                multiview_mask: None,
            }),
        );
    }

    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        queue: &Queue,
        device: &Device,
        global_context: &mut GlobalContext,
    ) {
        self.meshes
            .iter_mut()
            .for_each(|mesh| mesh.update(device, queue, global_context));
        if let Some(pp) = self.pipeline.as_ref() {
            render_pass.set_pipeline(pp);

            self.mesh_pipeline
                .render(render_pass, queue, global_context);
            self.meshes
                .iter_mut()
                .for_each(|mesh| mesh.render_kiol(render_pass));
        }
    }
}
