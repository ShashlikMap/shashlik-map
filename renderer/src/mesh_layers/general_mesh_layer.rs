use crate::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::modifier::render_modifier::SpatialData;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::pipelines::RenderPipeline;
use cgmath::Vector3;
use wgpu::{Device, RenderPass};

pub struct GeneralMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    render_data_holder: RenderDataHolder<PositionedMesh<P::InstanceInputType>>,
}

impl<P: RenderPipeline> GeneralMeshLayer<P> {
    pub fn new(render_pipeline: P) -> Self {
        GeneralMeshLayer {
            render_pipeline,
            pipeline: None,
            render_data_holder: RenderDataHolder::new(),
        }
    }
    pub fn add(
        &mut self,
        key: String,
        device: &Device,
        instance_positions: Option<Vec<Vector3<f64>>>,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
        with_collisions: bool,
        mesh: Mesh,
    ) {
        let mesh = P::create_positioned_mesh(
            device,
            instance_positions,
            spatial_rx,
            is_two_instances,
            with_collisions,
            mesh,
        );
        self.render_data_holder.add(key, mesh);
    }
}

impl<P: RenderPipeline> BaseMeshLayer for GeneralMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        self.render_data_holder
            .holder
            .iter_mut()
            .for_each(|(_, mesh)| mesh.update(global_context));
        if let Some(pp) = self.pipeline.as_ref() {
            render_pass.set_pipeline(pp);

            self.render_pipeline.render(render_pass, global_context);

            self.render_data_holder
                .holder
                .iter_mut()
                .for_each(|(_, mesh)| mesh.render(render_pass));
        }
    }

    fn clear_by_key(&mut self, key: String) {
        self.render_data_holder.remove(key);
    }
}
