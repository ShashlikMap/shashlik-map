use geo_types::Coord;
use glam::{DMat4, DVec2, DVec3, Mat4, Vec3, Vec4Swizzles};
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
use std::sync::{Arc, mpsc};
use std::time::Instant;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

/// This is the very beginning of CPU renderer-gpu. So far, just a stub animation

pub struct CpuRenderer {
    start_time: Instant,
    canvas_api: CpuCanvasApi,
    temp_shapes: Vec<(SpatialData, Vec<ShapeData>)>,
    receiver: Receiver<(SpatialData, Box<dyn RenderGroup<CpuCanvasApi>>)>,
    cpu_renderer_api: Arc<CpuRendererApi>,
    cs_offset: DVec3,
    inv_view_proj_matrix: DMat4,
    view_proj_matrix: DMat4,
}

pub struct CpuRendererApi {
    pub sender: Sender<(SpatialData, Box<dyn RenderGroup<CpuCanvasApi>>)>,
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
    fn set_feature_layer_tag(&mut self, tag: Option<String>) {}

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
        println!(
            "add_render_group = {:?}, with spat ={:?}",
            key, spatial_data
        );
        self.sender.send((spatial_data, group)).unwrap();
    }

    fn clear_render_groups(&self, keys: HashSet<String>) {}

    fn update_style<F: FnOnce(&mut RenderStyle) + Send + 'static>(
        &self,
        style_id: StyleId,
        updater: F,
    ) {
    }

    fn update_spatial_data<F: FnOnce(&mut SpatialData) + Send + 'static>(
        &self,
        key: String,
        updater: F,
    ) {
    }
}

impl CpuRenderer {
    pub const WIDTH: u32 = 600;
    pub const HEIGHT: u32 = 600;
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            start_time: Instant::now(),
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

    fn resize(&mut self, width: u32, height: u32) {}

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
            let (spatial_data, mut group) = msg;
            group.content(&mut self.canvas_api);

            self.temp_shapes
                .push((spatial_data, self.canvas_api.take_shapes()));
        }

        let mut pixmap = Pixmap::new(Self::WIDTH, Self::HEIGHT).unwrap();

        pixmap.fill(Color::from_rgba8(244, 243, 240, 255));

        let translation: DVec3 = self.view_proj_matrix.w_axis.xyz();

        let tile_width = 256.0;
        let tile_height = 144.29587 - 0.7179349;
        self.temp_shapes
            .iter()
            .for_each(|(spat_data, shapes_data)| {
                let qx = (spat_data.transform.x - self.cs_offset.x) as f32;
                let qy = (spat_data.transform.y - self.cs_offset.y) as f32;
                let xx = -tile_width - qx;
                let yy = -tile_height - qy;

                for shape_data in shapes_data {
                    let mut pb = PathBuilder::new();
                    let mut is_line = matches!(shape_data.geometry_type, GeometryType::Polyline { .. });
                    shape_data.path.iter().for_each(|path| match path {
                        PathEvent::Begin { at } => {
                            pb.move_to(at.x - xx, (-at.y + yy));
                        }
                        PathEvent::Line { from, to } => {
                            pb.line_to(to.x - xx, (-to.y + yy));
                        }
                        PathEvent::Quadratic { .. } => {}
                        PathEvent::Cubic { .. } => {}
                        PathEvent::End { .. } => {
                            if !is_line {
                                pb.close();
                            }
                        }
                    });
                    if let Some(path) = pb.finish() {
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
                            // println!("type = {:?}", shape_data.style_id);
                        }

                        paint.anti_alias = true;
                        let transform = Transform::from_scale(1.5, -1.5)
                            .post_translate(translation.x as f32, translation.y as f32);
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
