use crate::nodes::scene_tree::RenderContext;
use wgpu::{BindGroupLayout, BlendState, DepthStencilState, Device, Face, RenderPipeline, ShaderModule, TextureFormat, VertexBufferLayout};

#[derive(Clone, Debug)]
pub struct PipeLineProvider {
    texture_format: TextureFormat,
    depth_state: DepthStencilState,
    multisample_state: wgpu::MultisampleState
}

impl PipeLineProvider {
    pub fn new(texture_format: TextureFormat,
               depth_state: DepthStencilState,
               multisample_state: wgpu::MultisampleState) -> Self {
        Self {
            texture_format,
            depth_state,
            multisample_state
        }
    }

    pub fn create(
        &self,
        device: &Device,
        render_context: &mut RenderContext,
        buffer_layouts: &[VertexBufferLayout],
        shader_module: &ShaderModule,
        custom_cull_mode: Option<Face>,
    ) -> RenderPipeline {
        let bind_group_layouts: Vec<&BindGroupLayout> = render_context.bind_group_layouts.iter().collect();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: Some("vs_main"),
                buffers: buffer_layouts,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.texture_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // FrontFace::Cw since we flip Y during matrix calculation
                // So the 3D buildings look correct
                front_face: wgpu::FrontFace::Cw,
                cull_mode: custom_cull_mode,
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some({
                let mut depth_state = self.depth_state.clone();
                depth_state.depth_write_enabled = render_context.can_write_depth;
                depth_state.depth_compare = render_context.depth_compare;
                depth_state
            }),
            multisample: self.multisample_state,
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        })
    }
}
