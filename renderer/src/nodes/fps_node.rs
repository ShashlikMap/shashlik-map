use cgmath::{vec2, vec3};
use crate::fps::FpsCounter;
use crate::nodes::scene_tree::RenderContext;
use crate::nodes::SceneNode;
use crate::GlobalContext;
use rustybuzz::ttf_parser::Face;
use wgpu::{DepthStencilState, Device, Queue, RenderPass, SurfaceConfiguration};
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{OwnedSection, OwnedText};
use wgpu_text::{BrushBuilder, TextBrush};
use crate::text::text_renderer::TextNodeData;

pub struct FpsNode {
    text_brush: TextBrush<FontRef<'static>>,
    counter: FpsCounter<100>,
    text_section: OwnedSection,
    current_fps: String,
    node: TextNodeData,

}

impl FpsNode {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        depth_state: DepthStencilState,
        multi_sample_state: wgpu::MultisampleState,
        font: &'static Face
    ) -> Self {
        let mut depth_state = depth_state.clone();
        depth_state.depth_write_enabled = false;
        let text_brush = BrushBuilder::using_font_bytes(font.raw_face().data).unwrap()
            .with_depth_stencil(Some(depth_state))
            .with_multisample(multi_sample_state)
            .build(device, config.width, config.height, config.format);
        Self {
            text_brush,
            counter: FpsCounter::new(),
            text_section: OwnedSection::default().with_screen_position((130f32, 50f32)),
            current_fps: "0".to_string(),
            node: TextNodeData {
                id: 0,
                text: "KIOL".to_string(),
                size: 40.0,
                alpha: 1.0,
                positions: vec![vec3(500.0, 500.0, 0.0)],
                screen_offset: vec2(0.0, 0.0),
                screen_space: true,
                glyph_buffer: None,
            }
        }
    }
}

impl SceneNode for FpsNode {
    fn setup(&mut self, _render_context: &mut RenderContext, _device: &Device) {}

    fn update(
        &mut self,
        device: &Device,
        queue: &Queue,
        config: &wgpu::SurfaceConfiguration,
        global_context: &mut GlobalContext,
    ) {
        let qq = self.counter.update() as i32;
        let screen_position_calculator = global_context
            .view_projection
            .screen_position_calculator(&global_context.view_projection.cs_offset, config);

        global_context.text_renderer.insert(
            &mut TextNodeData {
                id: 0,
                text: format!("FPS {}", qq),
                size: 40.0,
                alpha: 1.0,
                positions: vec![vec3(100.0, 50.0, 0.0)],
                screen_offset: vec2(0.0, 0.0),
                screen_space: true,
                glyph_buffer: None,
            },
            &mut global_context.collision_handler,
            &screen_position_calculator,
        )

        // self.text_section.text.clear();
        // self.text_section
        //     .text
        //     .push(OwnedText::new(self.current_fps.as_str()).with_scale(60.0));
        //
        // self.text_brush
        //     .queue(&device, &queue, [&self.text_section])
        //     .unwrap();
    }

    fn render(&mut self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        // self.text_brush.draw(render_pass)
    }

    fn resize(&mut self, width: u32, height: u32, queue: &Queue) {
        // self.text_brush
        //     .resize_view(width as f32, height as f32, queue);
    }
}
