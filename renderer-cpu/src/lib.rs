use geo_types::{Coord, coord};
use glam::{DMat4, DVec2, DVec3};
use indexmap::IndexMap;
use lyon_algorithms::aabb::bounding_box;
use lyon_path::PathEvent;
use lyon_path::geom::point;
use lyon_path::math::Box2D;
pub use renderer_common::fps::FpsCounter;
use renderer_common::geometry_data::{GeometryData, GeometryType, ShapeData};
use renderer_common::r_api_messenger::{CommonRendererApi, RendererApiMsg};
use renderer_common::render_modifier::SpatialData;
use renderer_common::render_style::RenderStyle;
use renderer_common::style_id::StyleId;
use renderer_common::{min_f64, CanvasApi, Renderer, RendererUpdateData, max_f64};
use rustc_hash::FxHashMap;
use skia_safe::{Canvas, Color4f, Paint, PaintStyle, PathBuilder, PictureRecorder, Rect};
use std::mem;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, mpsc};

/// This is the very beginning of CPU renderer-gpu.
pub struct CpuRenderer {
    size: (u32, u32),
    canvas_api: CpuCanvasApi,
    shapes_background: Vec<(String, SpatialData, Vec<(Box2D, ShapeData)>)>,
    shapes_foreground: Vec<(String, SpatialData, Vec<(Box2D, ShapeData)>)>,
    shapes_features: IndexMap<String, Vec<(String, SpatialData, Vec<(Box2D, ShapeData)>)>>,
    receiver: Receiver<RendererApiMsg<CpuCanvasApi>>,
    cpu_renderer_api: Arc<CommonRendererApi<CpuCanvasApi>>,
    cs_offset: DVec3,
    inv_view_proj_matrix: DMat4,
    view_proj_matrix: DMat4,
    ground_style: StyleId,
    styles_map: FxHashMap<StyleId, Color4f>,
    spatial_map: FxHashMap<String, SpatialData>,
    norm_length: f64,
    screen_aabb: Box2D,
}

#[derive(Default)]
pub struct CpuCanvasApi {
    shapes: Vec<ShapeData>,
    feature_shapes: IndexMap<String, Vec<ShapeData>>,
    current_tag: Option<String>,
}

impl CpuCanvasApi {
    pub fn take_shapes(&mut self) -> Vec<ShapeData> {
        mem::take(&mut self.shapes)
    }

    pub fn take_feature_shapes(&mut self) -> IndexMap<String, Vec<ShapeData>> {
        self.current_tag = None;
        mem::take(&mut self.feature_shapes)
    }
}

impl CanvasApi for CpuCanvasApi {
    fn set_feature_layer_tag(&mut self, tag: Option<String>) {
        self.current_tag = tag;
    }

    fn geometry_data(&mut self, geometry_data: GeometryData) {
        match geometry_data {
            GeometryData::Shape(data) => {
                if let Some(tag) = self.current_tag.clone() {
                    self.feature_shapes.insert(tag, vec![data]);
                } else {
                    self.shapes.push(data);
                }
            }
            _ => {}
        }
    }
}

impl CpuRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            size: (width, height),
            canvas_api: Default::default(),
            shapes_background: vec![],
            shapes_foreground: vec![],
            shapes_features: Default::default(),
            receiver,
            cpu_renderer_api: Arc::new(CommonRendererApi::new(sender)),
            cs_offset: Default::default(),
            inv_view_proj_matrix: Default::default(),
            view_proj_matrix: Default::default(),
            ground_style: StyleId::new("ground"),
            styles_map: Default::default(),
            spatial_map: Default::default(),
            norm_length: 0.0,
            screen_aabb: Box2D::new(point(-1.0, -1.0), point(1.0, 1.0)),
        }
    }
}

impl CpuRenderer {
    const BLACK_FALLBACK: Color4f = Color4f::new(0.0, 0.0, 0.0, 1.0);
    const HAIRLINE_THRESHOLD: f32 = 1.0;
    #[inline]
    fn calc_normalized_vector_proj_length(&self) -> f64 {
        let center = self.clip_to_world(&coord! { x: 0.0, y: 0.0}).unwrap();
        let center_with_offset = center + 1.0;
        let projected_center = self.view_proj_matrix.project_point3(center.extend(0.0));
        let projected_center_offset = self
            .view_proj_matrix
            .project_point3(center_with_offset.extend(0.0));
        // TODO how to pass 200.0 koef from map?
        (projected_center_offset - projected_center).length() * 250.0
    }

    #[inline]
    fn process_shapes(
        size: (u32, u32),
        shapes: &[(String, SpatialData, Vec<(Box2D, ShapeData)>)],
        canvas: &Canvas,
        cs_offset: &DVec3,
        norm_length: f64,
        view_proj_matrix: &DMat4,
        screen_aabb: &Box2D,
        styles_map: &FxHashMap<StyleId, Color4f>,
        spatial_map: &FxHashMap<String, SpatialData>,
    ) {
        shapes.iter().for_each(|(key, spat_data, shapes_data)| {
            let external_spat_data = spatial_map.get(key);

            let project_point = |point| {
                let modified = if let Some(external_spat_data) = external_spat_data {
                    let scale_rot = external_spat_data.scale_rot_matrix();
                    scale_rot.project_point3(point) + external_spat_data.transform
                } else {
                    point
                };
                view_proj_matrix
                    .project_point3(modified + spat_data.transform - cs_offset)
                    .truncate()
            };

            for (aabb, shape_data) in shapes_data {
                let mut pb = PathBuilder::new();
                let mut is_line = false;
                let mut l_width = 0.0;
                match shape_data.geometry_type {
                    GeometryType::Polyline(options) => {
                        l_width = (options.width as f64 * norm_length) as f32;
                        is_line = true;
                        if l_width < Self::HAIRLINE_THRESHOLD {
                            continue;
                        }
                    }
                    GeometryType::Polygon => {}
                }

                let aabb_projected1 =
                    project_point(DVec3::new(aabb.min.x as f64, aabb.min.y as f64, 0.0));
                let aabb_projected2 =
                    project_point(DVec3::new(aabb.max.x as f64, aabb.min.y as f64, 0.0));
                let aabb_projected3 =
                    project_point(DVec3::new(aabb.min.x as f64, aabb.max.y as f64, 0.0));
                let aabb_projected4 =
                    project_point(DVec3::new(aabb.max.x as f64, aabb.max.y as f64, 0.0));
                let aabb_min_x = min_f64!(
                    aabb_projected1.x,
                    aabb_projected2.x,
                    aabb_projected3.x,
                    aabb_projected4.x
                );
                let aabb_max_x = max_f64!(
                    aabb_projected1.x,
                    aabb_projected2.x,
                    aabb_projected3.x,
                    aabb_projected4.x
                );
                let aabb_min_y = min_f64!(
                    aabb_projected1.y,
                    aabb_projected2.y,
                    aabb_projected3.y,
                    aabb_projected4.y
                );
                let aabb_max_y = max_f64!(
                    aabb_projected1.y,
                    aabb_projected2.y,
                    aabb_projected3.y,
                    aabb_projected4.y
                );

                let cond = Box2D::new(
                    point(aabb_min_x as f32, aabb_min_y as f32),
                    point(aabb_max_x as f32, aabb_max_y as f32),
                )
                .intersects(screen_aabb);
                if !cond {
                    continue;
                }
                shape_data.path.iter().for_each(|path| match path {
                    PathEvent::Begin { at } => {
                        let projected = project_point(DVec3::new(at.x as f64, at.y as f64, 0.0));
                        pb.move_to((
                            0.5 * (1.0 + projected.x as f32) * size.0 as f32,
                            0.5 * (1.0 + projected.y as f32) * size.1 as f32,
                        ));
                    }
                    PathEvent::Line { from: _, to } => {
                        let projected = project_point(DVec3::new(to.x as f64, to.y as f64, 0.0));
                        pb.line_to((
                            0.5 * (1.0 + projected.x as f32) * size.0 as f32,
                            0.5 * (1.0 + projected.y as f32) * size.1 as f32,
                        ));
                    }
                    PathEvent::Quadratic { .. } => {}
                    PathEvent::Cubic { .. } => {}
                    PathEvent::End { .. } => {
                        if !is_line {
                            pb.close();
                        }
                    }
                });
                let path = pb.detach();
                let mut paint = Paint::default();
                let color = styles_map
                    .get(&shape_data.style_id)
                    .unwrap_or_else(|| &Self::BLACK_FALLBACK);
                paint.set_color(color.to_color());
                paint.set_anti_alias(true);

                if is_line {
                    paint.set_stroke_width(l_width);
                    paint.set_style(PaintStyle::Stroke);
                } else {
                    paint.set_style(PaintStyle::Fill);
                }
                canvas.draw_path(&path, &paint);
            }
        });
    }
}

impl Renderer for CpuRenderer {
    type RAPI = CommonRendererApi<CpuCanvasApi>;
    type OUTPUT = ();
    type INPUT<'a> = &'a Canvas;

    fn screen_size(&self) -> (f32, f32) {
        (self.size.0 as f32, self.size.1 as f32)
    }

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn update(&mut self, data: RendererUpdateData) {
        self.cs_offset = data.cs_offset;
        self.view_proj_matrix = data.view_proj_matrix;
        self.inv_view_proj_matrix = data.view_proj_matrix.inverse();

        self.norm_length = self.calc_normalized_vector_proj_length();
    }

    fn clip_to_world(&self, coord: &Coord) -> Option<DVec2> {
        Self::clip_to_world_at_ground(&DVec2::new(coord.x, coord.y), &self.inv_view_proj_matrix)
            .map(|coord| coord + self.cs_offset.truncate())
    }

    fn render(&mut self, input: Self::INPUT<'_>) -> Option<Self::OUTPUT> {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                RendererApiMsg::RenderGroup(key, spat_data, mut group) => {
                    group.content(&mut self.canvas_api);
                    let mut shapes: Vec<_> = self
                        .canvas_api
                        .take_shapes()
                        .into_iter()
                        .map(|shape_data| {
                            let path_aabb = bounding_box(shape_data.path.iter());
                            (path_aabb, shape_data)
                        })
                        .collect();

                    let shapes_features: IndexMap<String, Vec<(Box2D, ShapeData)>> = self
                        .canvas_api
                        .take_feature_shapes()
                        .into_iter()
                        .map(|item| {
                            let shapes_with_bbox: Vec<_> = item
                                .1
                                .into_iter()
                                .map(|shape_data| {
                                    let path_aabb = bounding_box(shape_data.path.iter());
                                    (path_aabb, shape_data)
                                })
                                .collect();
                            (item.0, shapes_with_bbox)
                        })
                        .collect();

                    let background_shapes = shapes
                        .extract_if(.., |(_, shape_data)| {
                            matches!(shape_data.geometry_type, GeometryType::Polygon)
                        })
                        .collect();
                    let foreground_shapes = shapes;
                    self.shapes_background.push((
                        key.clone(),
                        spat_data.clone(),
                        background_shapes,
                    ));
                    self.shapes_foreground.push((
                        key.clone(),
                        spat_data.clone(),
                        foreground_shapes,
                    ));

                    shapes_features.into_iter().for_each(|(tag_key, data)| {
                        self.shapes_features.entry(tag_key).or_default().push((
                            key.clone(),
                            spat_data.clone(),
                            data,
                        ));
                    });
                }
                RendererApiMsg::ClearGroups(keys) => {
                    self.shapes_background.retain(|s| !keys.contains(&s.0));
                    self.shapes_foreground.retain(|s| !keys.contains(&s.0));
                    // TODO reset spatial_map too?
                    self.shapes_features.values_mut().for_each(|list| {
                        list.retain(|s| !keys.contains(&s.0));
                    });
                }
                RendererApiMsg::UpdateStyle(style_id, updater) => {
                    let mut style = RenderStyle::default();
                    updater(&mut style);
                    let fill_color = style.get_fill_color();

                    let fill_color =
                        Color4f::new(fill_color[0], fill_color[1], fill_color[2], fill_color[3]);
                    self.styles_map.insert(style_id, fill_color);
                }
                RendererApiMsg::UpdateSpatialData(key, updater) => {
                    let spatial_data = self.spatial_map.entry(key).or_insert(SpatialData::new());
                    updater(spatial_data);
                }
            }
        }

        let processor = |shapes: &[(String, SpatialData, Vec<(Box2D, ShapeData)>)],
                         canvas: &Canvas| {
            Self::process_shapes(
                self.size,
                shapes,
                canvas,
                &self.cs_offset,
                self.norm_length,
                &self.view_proj_matrix,
                &self.screen_aabb,
                &self.styles_map,
                &self.spatial_map,
            );
        };

        let (pb, pa) = rayon::join(
            || {
                let mut recorder = PictureRecorder::new();
                let canvas_back = recorder
                    .begin_recording(Rect::from_wh(self.size.0 as f32, self.size.1 as f32), false);
                let ground_color = self
                    .styles_map
                    .get(&self.ground_style)
                    .unwrap_or_else(|| &Self::BLACK_FALLBACK);
                canvas_back.clear(ground_color.to_color());
                processor(&self.shapes_background, canvas_back);
                recorder.finish_recording_as_picture(None).unwrap()
            },
            || {
                let mut recorder = PictureRecorder::new();
                let canvas_front = recorder
                    .begin_recording(Rect::from_wh(self.size.0 as f32, self.size.1 as f32), false);
                processor(&self.shapes_foreground, canvas_front);
                self.shapes_features.values().for_each(|list| {
                    processor(list, canvas_front);
                });
                recorder.finish_recording_as_picture(None).unwrap()
            },
        );

        input.draw_picture(&pb, None, None);
        input.draw_picture(&pa, None, None);

        None
    }

    fn api(&self) -> Arc<CommonRendererApi<CpuCanvasApi>> {
        self.cpu_renderer_api.clone()
    }
}
