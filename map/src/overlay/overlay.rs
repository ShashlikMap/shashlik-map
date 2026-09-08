use crate::CoordConverter;
use crate::overlay::ShapeType;
use crate::overlay::overlay_shape_group::OverlayShapeGroup;
use geo_types::Point;
use renderer_common::RendererApi;
use renderer_common::style_id::StyleId;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use rustc_hash::FxHashSet;

static OVERLAY_SHAPE_ID: AtomicUsize = AtomicUsize::new(0);
pub struct Overlay<RAPI: RendererApi> {
    api: Arc<RAPI>,
    feature_layer_tag: String,
    shape_ids: FxHashSet<String>,
}

impl<RAPI: RendererApi> Overlay<RAPI> {
    pub fn new(feature_layer_tag: String, api: Arc<RAPI>) -> Overlay<RAPI> {
        Overlay {
            api,
            feature_layer_tag,
            shape_ids: FxHashSet::default(),
        }
    }

    pub fn add_overlay_shape(
        &mut self,
        converter: CoordConverter,
        points: Vec<Point>,
        shape_type: ShapeType,
        fill_color: [f32; 3],
    ) -> Option<String> {
        if !shape_type.are_points_valid(&points) {
            return None;
        }
        let points: Vec<Point> = points.iter().map(|p| converter(p)).collect();
        let id = OVERLAY_SHAPE_ID.fetch_add(1, Ordering::Relaxed);
        let render_style = renderer_common::render_style::RenderStyle::fill([
            fill_color[0],
            fill_color[1],
            fill_color[2],
            1.0,
        ]);
        let unique_id = format!("overlay_shape_id_{}", id);
        self.shape_ids.insert(unique_id.clone());
        // TODO Underlying style storage now just grows, decide how to handle it better once draw api becomes more mature
        //  Either to remove styles or reuse it
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
        Some(unique_id)
    }

    pub fn remove_shape(&mut self, key: String) {
        if self.shape_ids.remove(&key) {
            self.api.clear_render_groups(HashSet::from_iter(vec![key]));
        }
    }
}
