use geo_types::Coord;
use glam::{DMat4, DVec2, DVec3};
use lyon_path::PathEvent;
use renderer_common::geometry_data::{GeometryData, GeometryType, ShapeData};
use renderer_common::render_group::RenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::render_style::RenderStyle;
use renderer_common::style_id::StyleId;
use renderer_common::{CanvasApi, Renderer, RendererApi, RendererUpdateData};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::collections::HashSet;
use std::mem;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// This is the very beginning of CPU renderer-gpu.

pub enum RendererApiMsg {
    RenderGroup(String, SpatialData, Box<dyn RenderGroup<CpuCanvasApi>>),
    UpdateStyle(StyleId, Box<dyn FnOnce(&mut RenderStyle) + Send>),
    ClearGroups(HashSet<String>),
}

pub struct CpuRenderer {
    canvas_api: CpuCanvasApi,
    temp_shapes: Vec<(String, SpatialData, Vec<ShapeData>)>,
    receiver: Receiver<RendererApiMsg>,
    cpu_renderer_api: Arc<CpuRendererApi>,
    cs_offset: DVec3,
    inv_view_proj_matrix: DMat4,
    view_proj_matrix: DMat4,
    ground_style: StyleId,
    styles_map: FxHashMap<StyleId, Color>,
}

pub struct CpuRendererApi {
    pub sender: Sender<RendererApiMsg>,
}

#[derive(Default)]
pub struct CpuCanvasApi {
    pub shapes: Vec<ShapeData>,
}

impl CpuCanvasApi {
    pub fn take_shapes(&mut self) -> Vec<ShapeData> {
        mem::take(&mut self.shapes)
    }
}

impl CanvasApi for CpuCanvasApi {
    fn set_feature_layer_tag(&mut self, _tag: Option<String>) {}

    fn geometry_data(&mut self, geometry_data: GeometryData) {
        match geometry_data {
            GeometryData::Shape(data) => {
                self.shapes.push(data);
            }
            _ => {}
        }
    }
}

impl RendererApi for CpuRendererApi {
    type CANVAS = CpuCanvasApi;

    fn add_render_group(
        &self,
        key: String,
        spatial_data: SpatialData,
        group: Box<dyn RenderGroup<Self::CANVAS>>,
    ) {
        self.sender
            .send(RendererApiMsg::RenderGroup(key, spatial_data, group))
            .unwrap();
    }

    fn clear_render_groups(&self, keys: HashSet<String>) {
        self.sender.send(RendererApiMsg::ClearGroups(keys)).unwrap();
    }

    fn update_style<F: FnOnce(&mut RenderStyle) + Send + 'static>(
        &self,
        style_id: StyleId,
        updater: F,
    ) {
        self.sender
            .send(RendererApiMsg::UpdateStyle(style_id, Box::new(updater)))
            .unwrap();
    }

    fn update_spatial_data<F: FnOnce(&mut SpatialData) + Send + 'static>(
        &self,
        _key: String,
        _updater: F,
    ) {
    }
}

impl CpuRenderer {
    pub const WIDTH: u32 = 1024;
    pub const HEIGHT: u32 = 600;
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            canvas_api: Default::default(),
            temp_shapes: vec![],
            receiver,
            cpu_renderer_api: Arc::new(CpuRendererApi { sender }),
            cs_offset: Default::default(),
            inv_view_proj_matrix: Default::default(),
            view_proj_matrix: Default::default(),
            ground_style: StyleId::new("ground"),
            styles_map: Default::default(),
        }
    }
}
impl Renderer for CpuRenderer {
    type RAPI = CpuRendererApi;
    type OUTPUT = Pixmap;

    fn screen_size(&self) -> (f32, f32) {
        (Self::WIDTH as f32, Self::HEIGHT as f32)
    }

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn update(&mut self, data: RendererUpdateData) {
        self.cs_offset = data.cs_offset;
        self.view_proj_matrix = data.view_proj_matrix;
        self.inv_view_proj_matrix = data.view_proj_matrix.inverse();
    }

    fn clip_to_world(&self, coord: &Coord) -> Option<DVec2> {
        Self::clip_to_world_at_ground(&DVec2::new(coord.x, coord.y), &self.inv_view_proj_matrix)
            .map(|coord| coord + self.cs_offset.truncate())
    }

    fn render(&mut self) -> Option<Self::OUTPUT> {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                RendererApiMsg::RenderGroup(key, spat_data, mut group) => {
                    group.content(&mut self.canvas_api);
                    self.temp_shapes
                        .push((key, spat_data, self.canvas_api.take_shapes()));
                }
                RendererApiMsg::ClearGroups(keys) => {
                    self.temp_shapes.retain(|s| !keys.contains(&s.0));
                }
                RendererApiMsg::UpdateStyle(style_id, updater) => {
                    let mut style = RenderStyle::default();
                    updater(&mut style);
                    let fill_color = style.get_fill_color();
                    let fill_color = Color::from_rgba(fill_color[0], fill_color[1], fill_color[2], fill_color[3]).unwrap();
                    self.styles_map.insert(style_id, fill_color);
                }
            }
        }

        // TODO Explore optimization: allocation and screen dividing
        let mut pixmap = Pixmap::new(Self::WIDTH, Self::HEIGHT).unwrap();

        let ground_color = self.styles_map.get(&self.ground_style).unwrap_or(&Color::BLACK);
        pixmap.fill(ground_color.clone());

        self.temp_shapes
            .iter()
            .for_each(|(_, spat_data, shapes_data)| {
                let spatial_offset = (spat_data.transform - self.cs_offset).truncate();

                for shape_data in shapes_data {
                    let mut pb = PathBuilder::new();
                    let mut is_line = false;
                    match shape_data.geometry_type {
                        GeometryType::Polyline(_) => {
                            is_line = true;
                        }
                        GeometryType::Polygon => {}
                    }
                    let mut not_culled = false;
                    shape_data.path.iter().for_each(|path| match path {
                        PathEvent::Begin { at } => {
                            let projected = self
                                .view_proj_matrix
                                .project_point3(DVec3::new(
                                    at.x as f64 + spatial_offset.x,
                                    at.y as f64 + spatial_offset.y,
                                    0.0,
                                ))
                                .truncate();
                            if projected.x >= -1.0
                                && projected.y >= -1.0
                                && projected.x <= 1.0
                                && projected.y <= 1.0
                            {
                                not_culled = true;
                            }
                            pb.move_to(
                                0.5 * (1.0 + projected.x as f32) * Self::WIDTH as f32,
                                0.5 * (1.0 + projected.y as f32) * Self::HEIGHT as f32,
                            );
                        }
                        PathEvent::Line { from: _, to } => {
                            let projected = self
                                .view_proj_matrix
                                .project_point3(DVec3::new(
                                    to.x as f64 + spatial_offset.x,
                                    to.y as f64 + spatial_offset.y,
                                    0.0,
                                ))
                                .truncate();
                            if projected.x >= -1.0
                                && projected.y >= -1.0
                                && projected.x <= 1.0
                                && projected.y <= 1.0
                            {
                                not_culled = true;
                            }
                            pb.line_to(
                                0.5 * (1.0 + projected.x as f32) * Self::WIDTH as f32,
                                0.5 * (1.0 + projected.y as f32) * Self::HEIGHT as f32,
                            );
                        }
                        PathEvent::Quadratic { .. } => {}
                        PathEvent::Cubic { .. } => {}
                        PathEvent::End { .. } => {
                            if !is_line {
                                pb.close();
                            }
                        }
                    });
                    if not_culled && let Some(path) = pb.finish() {
                        let mut paint = Paint::default();
                        let color = self.styles_map.get(&shape_data.style_id).unwrap_or(&Color::BLACK).clone();
                        paint.set_color(color);
                        paint.anti_alias = true;

                        if is_line {
                            pixmap.stroke_path(
                                &path,
                                &paint,
                                &Stroke::default(),
                                Transform::default(),
                                None,
                            )
                        } else {
                            pixmap.fill_path(
                                &path,
                                &paint,
                                tiny_skia::FillRule::EvenOdd,
                                Transform::default(),
                                None,
                            );
                        }
                    }
                }
            });

        Some(pixmap)
    }

    fn api(&self) -> Arc<CpuRendererApi> {
        self.cpu_renderer_api.clone()
    }
}
