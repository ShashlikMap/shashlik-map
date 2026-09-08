use crate::CoordConverter;
use crate::overlay::ShapeType;
use crate::overlay::overlay_shape_group::OverlayShapeGroup;
use geo_types::Point;
use renderer_common::RendererApi;
use renderer_common::style_id::StyleId;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

static OVERLAY_SHAPE_ID: AtomicUsize = AtomicUsize::new(0);
pub struct Overlay<RAPI: RendererApi> {
    api: Arc<RAPI>,
    feature_layer_tag: String,
}

impl<RAPI: RendererApi> Overlay<RAPI> {
    pub fn new(feature_layer_tag: String, api: Arc<RAPI>) -> Overlay<RAPI> {
        Overlay {
            api,
            feature_layer_tag,
        }
    }

    pub fn add_overlay_shape(
        &self,
        converter: CoordConverter,
        points: Vec<Point>,
        shape_type: ShapeType,
        fill_color: [f32; 3],
    ) -> String {
        let points: Vec<Point> = points.iter().map(|p| converter(p)).collect();
        let id = OVERLAY_SHAPE_ID.fetch_add(1, Ordering::Relaxed);
        let render_style = renderer_common::render_style::RenderStyle::fill([
            fill_color[0],
            fill_color[1],
            fill_color[2],
            1.0,
        ]);
        let unique_id = format!("overlay_shape_id_{}", id);
        let style_id = StyleId::new(unique_id.clone());
        self.api
            .update_style(style_id.clone(), move |style| *style = render_style);
        let shape = Box::new(OverlayShapeGroup::new(
            points,
            self.feature_layer_tag.clone(),
            style_id.clone(),
            shape_type,
        ));

        self.api
            .add_render_group(unique_id.clone(), shape.spatial_data(), shape);
        unique_id
    }

    pub fn remove_shape(&self, key: String) {
        self.api.clear_render_groups(HashSet::from_iter(vec![key]));
    }
}
