use crate::CoordConverter;
use crate::overlay::ShapeType;
use crate::overlay::overlay_shape_group::OverlayShapeGroup;
use geo_types::{Point, Rect};
use renderer_common::RendererApi;
use renderer_common::style_id::StyleId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use geo::{BoundingRect, Scale};
use geo_types::Geometry::MultiPoint;
use glam::DVec3;
use renderer_common::render_modifier::SpatialData;
use crate::puck_group::SimplePuck;

static OVERLAY_SHAPE_ID: AtomicUsize = AtomicUsize::new(0);
pub struct Overlay<RAPI: RendererApi> {
    api: Arc<RAPI>,
    feature_layer_tag: String,
    shape_ids: FxHashSet<String>,
    styles: FxHashMap<String, StyleId>,
    points: FxHashMap<String, Vec<Point>>,
    bbox: Option<Rect>,
    last_normal_scale: Option<f64>
}

impl<RAPI: RendererApi> Overlay<RAPI> {
    const BBOX_SCALE: f64 = 1.5;
    pub fn new(feature_layer_tag: String, api: Arc<RAPI>) -> Overlay<RAPI> {
        Overlay {
            api,
            feature_layer_tag,
            shape_ids: FxHashSet::default(),
            styles: FxHashMap::default(),
            points: FxHashMap::default(),
            bbox: None,
            last_normal_scale: None
        }
    }

    pub fn puck_config(&self, enabled: bool) {
        let puck_key = "puck".to_string();
        if enabled {
            let mut puck_spatial_data = SpatialData::transform(DVec3::new(0.0, 0.0, 0.0));
            puck_spatial_data.scale(DVec3::splat(1.0));
            self.api.add_render_group(
                puck_key,
                puck_spatial_data,
                Box::new(SimplePuck {}),
            );
        } else {
            self.api.clear_render_groups(HashSet::from_iter(vec![puck_key]))
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

        self.points.insert(unique_id.clone(), points.clone());
        self.bbox = None;

        self.shape_ids.insert(unique_id.clone());

        let style_key = format!("overlay_shape_key_{:?}", render_style);
        let style_id = self.styles.entry(style_key.clone()).or_insert_with(|| {
            let id = StyleId::new(style_key);
            self.api
                .update_style(id.clone(), move |style| *style = render_style);
            id
        });

        let shape = Box::new(OverlayShapeGroup::new(
            points,
            self.feature_layer_tag.clone(),
            style_id.clone(),
            shape_type,
        ));

        self.api
            .add_render_group(unique_id.clone(), shape.spatial_data(self.last_normal_scale), shape);
        Some(unique_id)
    }

    pub fn has_shapes(&self) -> bool {
        self.shape_ids.len() > 0
    }

    pub fn bbox(&mut self) -> Option<&Rect> {
        if self.bbox.is_none() && let Some(bbox) = MultiPoint(self.points.values().cloned().flatten().collect()).bounding_rect() {
            self.bbox = Some(bbox.scale(Self::BBOX_SCALE));
        }
        self.bbox.as_ref()
    }

    pub fn remove_shape(&mut self, key: String) {
        if self.points.remove(&key).is_some() {
            self.bbox = None;
        }
        if self.shape_ids.remove(&key) {
            self.api.clear_render_groups(HashSet::from_iter(vec![key]));
        }
    }

    pub fn update(&mut self, normal_scale: f64) {
        self.last_normal_scale = Some(normal_scale);
        let api = Arc::clone(&self.api);
        self.shape_ids.iter().for_each(|shape_id| {
            api.update_spatial_data(shape_id.clone(), move |spatial_data| {
                spatial_data.normal_scale = normal_scale;
            });
        })
    }
}
