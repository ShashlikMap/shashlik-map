use renderer::renderer_api::RendererApi;
use renderer::styles::render_style::RenderStyle;
use renderer::styles::style_id::StyleId;
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::Duration;

pub struct StyleLoader {}

impl StyleLoader {
    pub fn new() -> StyleLoader {
        StyleLoader {}
    }

    pub fn load(&self, api: Arc<RendererApi>) {
        spawn(move || {
            // simulate loading
            sleep(Duration::from_millis(1000));
            let new_styles = vec![
                (StyleId("poi"), RenderStyle::fill([0.0, 0.0, 1.0, 1.0])),
                (
                    StyleId("road"),
                    RenderStyle::border([0.87843, 0.48627, 0.56471, 1.0], 0.5),
                ),
                (
                    StyleId("water"),
                    RenderStyle::fill([0.0, 0.741, 0.961, 1.0]),
                ),
                (
                    StyleId("building"),
                    RenderStyle::border([0.5, 0.5, 0.5, 1.0], 0.0),
                ),
                (
                    StyleId("land"),
                    RenderStyle::fill([0.447, 0.91, 0.651, 1.0]),
                ),
                (
                    StyleId("puck_style"),
                    RenderStyle::fill([0.0, 0.09, 1.0, 1.0]),
                ),
            ];

            new_styles.into_iter().for_each(|(style_id, render_style)| {
                api.update_style(style_id, move |style| {
                    *style = render_style
                });
            });
        });
    }
}
