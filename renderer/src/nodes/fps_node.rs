use crate::GlobalContext;
use crate::fps::FpsCounter;
use crate::nodes::SceneNode;
use crate::nodes::scene_tree::RenderContext;
use wgpu::{DepthStencilState, Device, Queue, RenderPass, SurfaceConfiguration};
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{OwnedSection, OwnedText};
use wgpu_text::{BrushBuilder, TextBrush};

pub struct FpsNode {
    text_brush: TextBrush<FontRef<'static>>,
    counter: FpsCounter<100>,
    text_section: OwnedSection,
    current_fps: String,
}

impl FpsNode {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        depth_state: DepthStencilState,
        multi_sample_state: wgpu::MultisampleState,
    ) -> Self {
        Self {
            text_brush: Self::create_default_text_brush(
                device,
                config,
                depth_state,
                multi_sample_state,
            ),
            counter: FpsCounter::new(),
            text_section: OwnedSection::default().with_screen_position((130f32, 50f32)),
            current_fps: "0".to_string(),
        }
    }

    fn create_default_text_brush<'a>(
        device: &Device,
        config: &SurfaceConfiguration,
        depth_state: DepthStencilState,
        multi_sample_state: wgpu::MultisampleState,
    ) -> TextBrush<FontRef<'a>> {
        let mut depth_state = depth_state.clone();
        depth_state.depth_write_enabled = false;
        BrushBuilder::using_font_bytes(include_bytes!("../font.ttf"))
            .unwrap()
            .with_depth_stencil(Some(depth_state))
            .with_multisample(multi_sample_state)
            .build(device, config.width, config.height, config.format)
    }
}

impl SceneNode for FpsNode {
    fn setup(&mut self, _render_context: &mut RenderContext, _device: &Device) {}

    fn update(
        &mut self,
        device: &Device,
        queue: &Queue,
        _config: &wgpu::SurfaceConfiguration,
        _global_context: &mut GlobalContext,
    ) {
        self.current_fps = format!("{:.1}", self.counter.update());

        self.text_section.text.clear();
        self.text_section
            .text
            .push(OwnedText::new(self.current_fps.as_str()).with_scale(60.0));

        self.text_brush
            .queue(&device, &queue, [&self.text_section])
            .unwrap();
    }

    fn render(&mut self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        self.text_brush.draw(render_pass)
    }

    fn resize(&mut self, width: u32, height: u32, queue: &Queue) {
        self.text_brush.resize_view(width as f32, height as f32, queue);
    }
}
