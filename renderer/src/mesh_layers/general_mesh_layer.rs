use crate::mesh::mesh::Mesh;
use crate::mesh_layers::BaseMeshLayer;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::mesh_node::PositionedMesh;
use crate::nodes::SceneNode;
use crate::pipelines::RenderPipeline;
use crate::GlobalContext;
use cgmath::Vector3;
use wgpu::{Device, RenderPass};

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
        instance_positions: Option<Vec<Vector3<f64>>>,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
        with_collisions: bool,
        mesh: Mesh,
    ) {
        self.meshes
            .push(P::create_positioned_mesh(device, instance_positions, spatial_rx, is_two_instances, with_collisions, mesh));
    }
}

impl<P: RenderPipeline> BaseMeshLayer for GeneralMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        global_context: &mut GlobalContext,
    ) {
        self.meshes
            .iter_mut()
            .for_each(|mesh| mesh.update(global_context));
        if let Some(pp) = self.pipeline.as_ref() {
            render_pass.set_pipeline(pp);

            self.render_pipeline
                .render(render_pass, global_context);

            self.meshes
                .iter_mut()
                .for_each(|mesh| mesh.render_kiol(render_pass));
        }
    }
}
