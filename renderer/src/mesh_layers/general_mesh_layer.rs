use crate::draw_commands::mesh2d_draw_command::Mesh2dCommandBatch;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::mesh_layers::BaseMeshLayer;
use crate::modifier::render_modifier::SpatialData;
use crate::pipelines::RenderPipeline;
use std::mem;
use wgpu::RenderPass;

pub(crate) struct GeneralMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    render_data_holder: RenderDataHolder<PositionedMesh<P::InstanceInputType>>,
    pub disable_skip_mesh_feature: bool
}

impl<P: RenderPipeline> GeneralMeshLayer<P> {
    pub fn new(render_pipeline: P) -> Self {
        GeneralMeshLayer {
            render_pipeline,
            pipeline: None,
            render_data_holder: RenderDataHolder::new(),
            disable_skip_mesh_feature: false
        }
    }
    pub fn add(
        &mut self,
        key: String,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
        mesh: Mesh,
    ) {

        let mesh = P::create_positioned_mesh(
            spatial_rx,
            double_style,
            None,
            mesh,
            false
        );
        self.render_data_holder.set(key, vec![mesh]);
    }

    pub fn submit_batch(
        &mut self,
        key: &str,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        global_context: &mut GlobalContext,
        batch: &mut Mesh2dCommandBatch,
    ) {
        let device = global_context.device();
        let mesh = Mesh::create_layered(
            &device,
            &batch.mesh,
            mem::take(&mut batch.layers_indices),
        );

        let instance_positions =
            mem::take(&mut batch.mesh_info.instance_positions).map(|pos_items| pos_items.into_iter().map(|item| {
                (item, 1f32)
            }).collect());

        // let dots = batch.mesh_info.instance_positions.as_ref().unwrap_or(&vec![]).len();
        // let dots = &instance_positions.unwrap().len();
        let mesh = P::create_positioned_mesh(
            spatial_rx,
            batch.mesh_info.double_style,
            instance_positions,
            mesh,
            key.contains("route")
        );
        self.render_data_holder.set(key.to_string(), vec![mesh]);
    }
}

impl<P: RenderPipeline> BaseMeshLayer for GeneralMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.render_data_holder.run_mut_action(|mesh| {
            mesh.update(global_context);
        });
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if let Some(render_pipeline) = self.pipeline.as_ref() {
            render_pass.set_pipeline(render_pipeline);

            self.render_pipeline.render(render_pass, global_context);

            self.render_data_holder.run_mut_action(|mesh| {
                mesh.render(render_pass, global_context, self.disable_skip_mesh_feature);
            });
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        self.render_data_holder.remove(key);
    }
}
