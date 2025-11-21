use crate::fps::FpsCounter;
use crate::nodes::scene_tree::RenderContext;
use crate::nodes::SceneNode;
use crate::GlobalContext;
use wgpu::{Device, Queue, RenderPass};
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{OwnedSection, OwnedText};
use wgpu_text::TextBrush;

pub struct FpsNode {
    text_brush: TextBrush<FontRef<'static>>,
    counter: FpsCounter<100>,
    text_section: OwnedSection,
    current_fps: String,
}

impl FpsNode {
    pub fn new(text_brush: TextBrush<FontRef<'static>>) -> Self {
        Self {
            text_brush,
            counter: FpsCounter::new(),
            text_section: OwnedSection::default().with_screen_position((50f32, 50f32)),
            current_fps: "0".to_string(),
        }
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

    fn render(&self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        // self.text_brush.draw(render_pass)
    }
}
