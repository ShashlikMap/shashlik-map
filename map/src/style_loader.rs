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
            sleep(Duration::from_millis(1500));

            let new_styles = vec![
                (StyleId("poi"), RenderStyle::fill([0.0, 0.0, 1.0, 1.0])),
                (StyleId("poi_traffic_light"), RenderStyle::fill([0.0, 0.0, 0.0, 1.0])),
                (StyleId("poi_toilet"), RenderStyle::fill([0.6, 0.0, 0.6, 1.0])),
                (StyleId("kml_dots"), RenderStyle::fill([1.0, 0.0, 0.0, 1.0])),
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
                    StyleId("ground"),
                    RenderStyle::fill([0.52, 0.37, 0.29, 1.0]),
                ),
                (
                    StyleId("puck_style"),
                    RenderStyle::fill([0.0, 0.09, 1.0, 1.0]),
                ),
                (
                    StyleId("highway_motorway"),
                    RenderStyle::border([0.87843, 0.48627, 0.56471, 1.0], 0.3),
                ),
                (
                    StyleId("highway_primary"),
                    RenderStyle::border([0.98824, 0.83922, 0.64314, 1.0], 0.3),
                ),
                (
                    StyleId("highway_trunk"),
                    RenderStyle::border([0.97647, 0.69804, 0.61176, 1.0], 0.3),
                ),
                (
                    StyleId("highway_secondary"),
                    RenderStyle::border([0.97255, 0.98039, 0.77255, 1.0], 0.3),
                ),
                (
                    StyleId("highway_tertiary"),
                    RenderStyle::border([1.0, 1.0, 1.0, 1.0], 0.3),
                ),
                (
                    StyleId("highway_footway"),
                    RenderStyle::border([0.8, 0.0, 0.0, 1.0], 0.3),
                ),
                (
                    StyleId("highway_default"),
                    RenderStyle::border([1.0, 1.0, 1.0, 1.0], 0.3),
                ),
                (
                    StyleId("admin_line"),
                    RenderStyle::fill([0.0, 0.0, 0.0, 1.0]),
                ),
                (
                    StyleId("rails"),
                    RenderStyle::dashed([1.0, 1.0, 1.0, 1.0], [0.2, 0.2, 0.2, 1.0]),
                ),
            ];

            new_styles.into_iter().for_each(|(style_id, render_style)| {
                api.update_style(style_id, move |style| *style = render_style);
            });
        });
    }
}
