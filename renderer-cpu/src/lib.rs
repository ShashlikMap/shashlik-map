use geo_types::Coord;
use glam::{DMat4, DVec2, DVec3};
use lyon::path::PathEvent;
use renderer_common::geometry_data::{GeometryData, GeometryType, ShapeData};
use renderer_common::render_group::RenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::render_style::RenderStyle;
use renderer_common::style_id::StyleId;
use renderer_common::{CanvasApi, Renderer, RendererApi, RendererUpdateData};
use std::collections::HashSet;
use std::mem;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// This is the very beginning of CPU renderer-gpu.

pub enum RendererApiMsg {
    RenderGroup((String, SpatialData, Box<dyn RenderGroup<CpuCanvasApi>>)),
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
            .send(RendererApiMsg::RenderGroup((key, spatial_data, group)))
            .unwrap();
    }

    fn clear_render_groups(&self, keys: HashSet<String>) {
        self.sender.send(RendererApiMsg::ClearGroups(keys)).unwrap();
    }

    fn update_style<F: FnOnce(&mut RenderStyle) + Send + 'static>(
        &self,
        _style_id: StyleId,
        _updater: F,
    ) {
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
                RendererApiMsg::RenderGroup(mut group) => {
                    group.2.content(&mut self.canvas_api);
                    self.temp_shapes
                        .push((group.0, group.1, self.canvas_api.take_shapes()));
                }
                RendererApiMsg::ClearGroups(keys) => {
                    self.temp_shapes.retain(|s| !keys.contains(&s.0));
                }
            }
        }

        let mut pixmap = Pixmap::new(Self::WIDTH, Self::HEIGHT).unwrap();

        pixmap.fill(Color::from_rgba8(244, 243, 240, 255));

        let transform = Transform::from_scale(0.5, 0.5)
            .post_translate((Self::WIDTH as f32) * 0.5, (Self::HEIGHT as f32) * 0.5);
        let unclip = DVec2::new(Self::WIDTH as f64, Self::HEIGHT as f64);

        let hw = (Self::WIDTH / 1) as f64;
        let hh = (Self::HEIGHT / 1) as f64;
        self.temp_shapes
            .iter()
            .for_each(|(_, spat_data, shapes_data)| {
                let spatial_offset = (spat_data.transform - self.cs_offset).truncate();

                for shape_data in shapes_data {
                    let mut pb = PathBuilder::new();
                    let mut is_line =
                        matches!(shape_data.geometry_type, GeometryType::Polyline { .. });
                    let mut not_culled = false;
                    shape_data.path.iter().for_each(|path| match path {
                        PathEvent::Begin { at } => {
                            let projected = self.view_proj_matrix.project_point3(DVec3::new(
                                at.x as f64 + spatial_offset.x,
                                at.y as f64 + spatial_offset.y,
                                0.0,
                            )).truncate() * unclip;
                            if projected.x >= -hw && projected.y >= -hh && projected.x <= hw && projected.y <= hh {
                                not_culled = true;
                            }
                            pb.move_to(projected.x as f32, projected.y as f32);
                        }
                        PathEvent::Line { from: _, to } => {
                            let projected = self.view_proj_matrix.project_point3(DVec3::new(
                                to.x as f64 + spatial_offset.x,
                                to.y as f64 + spatial_offset.y,
                                0.0,
                            )).truncate() * unclip;
                            if projected.x >= -hw && projected.y >= -hh && projected.x <= hw && projected.y <= hh {
                                not_culled = true;
                            }
                            pb.line_to(projected.x as f32, projected.y as f32);
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
                        if shape_data.style_id.0 == "building_stand" {
                            is_line = false;
                            paint.set_color_rgba8(206, 208, 209, 255);
                        } else if shape_data.style_id.0 == "water" {
                            paint.set_color_rgba8(165, 201, 235, 255);
                        } else if shape_data.style_id.0 == "forest" {
                            paint.set_color_rgba8(193, 232, 200, 255);
                        } else if shape_data.style_id.0 == "park" {
                            paint.set_color_rgba8(209, 241, 215, 255);
                        } else if shape_data.style_id.0 == "ground" {
                            paint.set_color_rgba8(244, 243, 240, 255);
                        } else {
                            paint.set_color_rgba8(159, 158, 156, 255);
                        }

                        paint.anti_alias = true;

                        if is_line {
                            pixmap.stroke_path(&path, &paint, &Stroke::default(), transform, None)
                        } else {
                            pixmap.fill_path(
                                &path,
                                &paint,
                                tiny_skia::FillRule::Winding,
                                transform,
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
