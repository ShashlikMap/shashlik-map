use crate::consts::STYLE_SHADER_PARAMS_COUNT;
use crate::nodes::scene_tree::RenderContext;
use crate::nodes::SceneNode;
use crate::{GlobalContext, ReceiverExt};
use tokio::sync::broadcast::Receiver;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, CompareFunction, Device, Queue, RenderPass,
};

pub struct StyleAdapterNode<T: SceneNode> {
    scene_node: T,
    shader_group_index: u32,
    styles_bind_group_layout: BindGroupLayout,
    style_bind_group: Option<BindGroup>,
    depth_compare: CompareFunction,
    style_uniform_rx: Receiver<Vec<[f32; STYLE_SHADER_PARAMS_COUNT]>>,
}

impl<T: SceneNode> StyleAdapterNode<T> {
    pub fn new(
        device: &Device,
        style_uniform_rx: Receiver<Vec<[f32; STYLE_SHADER_PARAMS_COUNT]>>,
        scene_node: T,
        shader_group_index: u32,
        depth_compare: CompareFunction,
    ) -> Self {
        let styles_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("styles_bind_group_layout"),
            });
        StyleAdapterNode {
            scene_node,
            shader_group_index,
            styles_bind_group_layout,
            style_bind_group: None,
            depth_compare,
            style_uniform_rx: style_uniform_rx,
        }
    }
}

impl<T: SceneNode> SceneNode for StyleAdapterNode<T> {
    fn setup(&mut self, render_context: &mut RenderContext, device: &Device) {
        render_context.can_write_depth = false;
        render_context.depth_compare = self.depth_compare;
        render_context
            .bind_group_layouts
            .push(self.styles_bind_group_layout.clone());
        self.scene_node.setup(render_context, device);
    }

    fn update(
        &mut self,
        device: &Device,
        _queue: &Queue,
        _config: &wgpu::SurfaceConfiguration,
        _global_context: &mut GlobalContext,
    ) {
        if let Ok(uniforms) = self.style_uniform_rx.no_lagged() {
            // TODO We could reuse the buffer if styles count has not changed
            let styles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Style Buffer"),
                contents: bytemuck::cast_slice(&uniforms),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            let styles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.styles_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: styles_buffer.as_entire_binding(),
                }],
                label: Some("styles_bind_group"),
            });

            self.style_bind_group = Some(styles_bind_group);
        }
    }

    fn render(&self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if let Some(bind_group) = self.style_bind_group.as_ref() {
            render_pass.set_bind_group(self.shader_group_index, bind_group, &[]);
        }

        self.scene_node.render(render_pass, global_context);
    }
}
