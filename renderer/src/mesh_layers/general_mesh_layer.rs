use crate::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::BaseMeshLayer;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::SceneNode;
use crate::nodes::mesh_node::PositionedMesh;
use crate::pipelines::RenderPipeline;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};

pub struct GeneralMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    meshes: Vec<PositionedMesh<P::InstanceInputType>>,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl<P: RenderPipeline> GeneralMeshLayer<P> {
    pub fn new(render_pipeline: P) -> Self {
        GeneralMeshLayer {
            render_pipeline,
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
            .push(P::create_positioned_mesh(device, spatial_rx, mesh));
    }
}

impl<P: RenderPipeline> BaseMeshLayer for GeneralMeshLayer<P> {
    fn prepare(&mut self, device: &Device, config: &SurfaceConfiguration) {
        let descriptor = self.render_pipeline.prepare(device, config);
        self.pipeline = Some(descriptor.to_render_pipeline(device));
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

            self.render_pipeline
                .render(render_pass, queue, global_context);
            self.meshes
                .iter_mut()
                .for_each(|mesh| mesh.render_kiol(render_pass));
        }
    }
}
