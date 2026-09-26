use crate::CoordConverter;
use crate::overlay::ShapeType;
use crate::overlay::overlay_shape_group::OverlayShapeGroup;
use crate::puck_group::SimplePuck;
use geo::{BoundingRect, Scale, Translate};
use geo_types::{GeometryCollection, MultiPoint, Point, Rect};
use glam::{DVec2, DVec3};
use renderer_common::RendererApi;
use renderer_common::render_modifier::SpatialData;
use renderer_common::style_id::StyleId;
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

static OVERLAY_SHAPE_ID: AtomicUsize = AtomicUsize::new(0);

struct ShapeValue {
    shape_type: ShapeType,
    anchor_distance: Option<f64>
}
pub struct Overlay<RAPI: RendererApi> {
    api: Arc<RAPI>,
    feature_layer_tag: String,
    shapes: FxHashMap<String, ShapeValue>,
    styles: FxHashMap<String, StyleId>,
    rects: FxHashMap<String, Rect<f64>>,
    bbox: Option<Rect>,
    last_normal_scale: Option<f64>
}

impl<RAPI: RendererApi> Overlay<RAPI> {
    const BBOX_SCALE: f64 = 1.5;
    pub fn new(feature_layer_tag: String, api: Arc<RAPI>) -> Overlay<RAPI> {
        Overlay {
            api,
            feature_layer_tag,
            shapes: FxHashMap::default(),
            styles: FxHashMap::default(),
            rects: FxHashMap::default(),
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
        anchor: Option<Point>,
        shape_type: ShapeType,
        fill_color: [f32; 3],
    ) -> Option<String> {
        if !shape_type.are_points_valid(&points) {
            return None;
        }
        let points: Vec<Point> = if anchor.is_some() {
            points
        } else {
            points.iter().map(|p| converter(p)).collect()
        };
        let anchor = anchor.map(|p| {
            let p = converter(&p);
            DVec3::new(p.x(), p.y(), 0.0)
        });
        let anchor_distance = anchor.map(|_| {
            // This is a workaround to calculate scaling, the proper normals has to be created for polygons later
            DVec2::new(points[0].x(), points[0].y()).length()
        });
        let id = OVERLAY_SHAPE_ID.fetch_add(1, Ordering::Relaxed);
        let render_style = renderer_common::render_style::RenderStyle::fill([
            fill_color[0],
            fill_color[1],
            fill_color[2],
            1.0,
        ]);
        let unique_id = format!("overlay_shape_id_{}", id);

        let bbox_offset = anchor.unwrap_or(DVec3::splat(0.0));
        let bbox = MultiPoint(points.clone()).bounding_rect().unwrap().translate(bbox_offset.x, bbox_offset.y);
        self.rects.insert(unique_id.clone(), bbox);
        self.bbox = None;

        self.shapes.insert(unique_id.clone(), ShapeValue {
            shape_type,
            anchor_distance
        });

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
            anchor,
            anchor_distance,
            self.last_normal_scale
        ));

        self.api
            .add_render_group(unique_id.clone(), shape.spatial_data(), shape);

        Some(unique_id)
    }

    pub fn has_shapes(&self) -> bool {
        self.shapes.len() > 0
    }

    pub fn bbox(&mut self) -> Option<&Rect> {
        if self.bbox.is_none() && let Some(bbox) = {
            let geometry = GeometryCollection::from(self.rects.values().cloned().collect::<Vec<_>>());
            geometry.bounding_rect()
        } {
            self.bbox = Some(bbox.scale(Self::BBOX_SCALE));
        }
        self.bbox.as_ref()
    }

    pub fn remove_shape(&mut self, key: String) {
        if self.rects.remove(&key).is_some() {
            self.bbox = None;
        }
        if self.shapes.remove(&key).is_some() {
            self.api.clear_render_groups(HashSet::from_iter(vec![key]));
        }
    }

    pub fn update_spatial_data<F: FnOnce(&mut SpatialData) + Send + 'static>(
        &self,
        key: String,
        updater: F,
    ) {
        self.api.update_spatial_data(key, updater);
        // TODO Potentially we need to reset bbox
    }

    pub fn update(&mut self, normal_scale: f64) {
        self.last_normal_scale = Some(normal_scale);
        let api = Arc::clone(&self.api);
        self.shapes.iter().for_each(|(shape_id, shape)| {
            let is_polygon = matches!(shape.shape_type, ShapeType::Polygon);
            if !is_polygon || shape.anchor_distance.is_some() {
                let anchor_distance = shape.anchor_distance.unwrap_or(1.0);
                api.update_spatial_data(shape_id.clone(), move |spatial_data| {
                    spatial_data.normal_scale = normal_scale;
                    if is_polygon {
                        let anchor_scale = (anchor_distance + normal_scale) / anchor_distance;
                        spatial_data.scale = DVec3::splat(anchor_scale);
                    }
                });
            }
        })
    }
}