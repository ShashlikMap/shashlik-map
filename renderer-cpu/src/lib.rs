use geo_types::{coord, Coord};
use glam::{DMat4, DVec2, DVec3};
use lyon_algorithms::aabb::bounding_box;
use lyon_path::geom::point;
use lyon_path::math::Box2D;
use lyon_path::PathEvent;
pub use renderer_common::fps::FpsCounter;
use renderer_common::geometry_data::{GeometryData, GeometryType, ShapeData};
use renderer_common::render_group::RenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::render_style::RenderStyle;
use renderer_common::style_id::StyleId;
use renderer_common::{CanvasApi, Renderer, RendererApi, RendererUpdateData};
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::mem;
use skia_safe::{Canvas, Color, Color4f, Paint, PaintStyle, PathBuilder};
// use tiny_skia::{Color, Paint, PathBuilder, Pixmap, PixmapMut, PremultipliedColorU8, Stroke, Transform};

/// This is the very beginning of CPU renderer-gpu.

pub enum RendererApiMsg {
    RenderGroup(String, SpatialData, Box<dyn RenderGroup<CpuCanvasApi>>),
    UpdateStyle(StyleId, Box<dyn FnOnce(&mut RenderStyle) + Send>),
    ClearGroups(HashSet<String>),
}

pub struct CpuRenderer {
    canvas_api: CpuCanvasApi,
    shapes_background: Vec<(String, SpatialData, Vec<(Box2D, ShapeData)>)>,
    shapes_foreground: Vec<(String, SpatialData, Vec<(Box2D, ShapeData)>)>,
    receiver: Receiver<RendererApiMsg>,
    cpu_renderer_api: Arc<CpuRendererApi>,
    cs_offset: DVec3,
    inv_view_proj_matrix: DMat4,
    view_proj_matrix: DMat4,
    ground_style: StyleId,
    styles_map: FxHashMap<StyleId, Color4f>,
    norm_length: f64,
    screen_aabb: Box2D,
    // pixmap_fore: Pixmap,
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
        // let pixmap_foreground = Pixmap::new(Self::WIDTH, Self::HEIGHT).unwrap();
        Self {
            canvas_api: Default::default(),
            shapes_background: vec![],
            shapes_foreground: vec![],
            receiver,
            cpu_renderer_api: Arc::new(CpuRendererApi { sender }),
            cs_offset: Default::default(),
            inv_view_proj_matrix: Default::default(),
            view_proj_matrix: Default::default(),
            ground_style: StyleId::new("ground"),
            styles_map: Default::default(),
            norm_length: 0.0,
            screen_aabb: Box2D::new(point(-1.0, -1.0), point(1.0, 1.0)),
            // pixmap_fore: pixmap_foreground
        }
    }
}

impl CpuRenderer {
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

    // #[inline]
    // pub fn fast_blend(background: &mut Pixmap, foreground: &Pixmap) {
    //     assert_eq!(background.width(), foreground.width());
    //     assert_eq!(background.height(), foreground.height());
    //
    //     let bg_pixels = background.pixels_mut();
    //     let fg_pixels = foreground.pixels();
    //
    //     for (bg, fg) in bg_pixels.iter_mut().zip(fg_pixels.iter()) {
    //         if fg.alpha() == 0 {
    //             continue;
    //         }
    //
    //         if fg.alpha() == 255 {
    //             *bg = *fg;
    //             continue;
    //         }
    //
    //         let alpha_inv = 255 - fg.alpha() as u32;
    //
    //         let r = fg.red() as u32 + ((bg.red() as u32 * alpha_inv + 128) / 255);
    //         let g = fg.green() as u32 + ((bg.green() as u32 * alpha_inv + 128) / 255);
    //         let b = fg.blue() as u32 + ((bg.blue() as u32 * alpha_inv + 128) / 255);
    //         let a = fg.alpha() as u32 + ((bg.alpha() as u32 * alpha_inv + 128) / 255);
    //
    //         *bg = PremultipliedColorU8::from_rgba(r as u8, g as u8, b as u8, a as u8).unwrap();
    //     }
    // }
    //
    // #[inline]
    // pub fn fast_blend2(background: &mut PixmapMut, foreground: &Pixmap) {
    //     assert_eq!(background.width(), foreground.width());
    //     assert_eq!(background.height(), foreground.height());
    //
    //     let bg_pixels = background.pixels_mut();
    //     let fg_pixels = foreground.pixels();
    //
    //     for (bg, fg) in bg_pixels.iter_mut().zip(fg_pixels.iter()) {
    //         if fg.alpha() == 0 {
    //             continue;
    //         }
    //
    //         if fg.alpha() == 255 {
    //             *bg = *fg;
    //             continue;
    //         }
    //
    //         let alpha_inv = 255 - fg.alpha() as u32;
    //
    //         let r = fg.red() as u32 + ((bg.red() as u32 * alpha_inv + 128) / 255);
    //         let g = fg.green() as u32 + ((bg.green() as u32 * alpha_inv + 128) / 255);
    //         let b = fg.blue() as u32 + ((bg.blue() as u32 * alpha_inv + 128) / 255);
    //         let a = fg.alpha() as u32 + ((bg.alpha() as u32 * alpha_inv + 128) / 255);
    //
    //         *bg = PremultipliedColorU8::from_rgba(r as u8, g as u8, b as u8, a as u8).unwrap();
    //     }
    // }

    #[inline]
    fn process_shapes(
        shapes: &[(String, SpatialData, Vec<(Box2D, ShapeData)>)],
        canvas: &Canvas,
        cs_offset: &DVec3,
        norm_length: f64,
        view_proj_matrix: &DMat4,
        screen_aabb: &Box2D,
        styles_map: &FxHashMap<StyleId, Color4f>,
    ) {
        shapes.iter().for_each(|(_, spat_data, shapes_data)| {
            let spatial_offset = (spat_data.transform - cs_offset).truncate();

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
                // TODO There might be an issue with rotation, check all corners?
                let projected_min = view_proj_matrix
                    .project_point3(DVec3::new(
                        aabb.min.x as f64 + spatial_offset.x,
                        aabb.min.y as f64 + spatial_offset.y,
                        0.0,
                    ))
                    .truncate();
                let projected_max = view_proj_matrix
                    .project_point3(DVec3::new(
                        aabb.max.x as f64 + spatial_offset.x,
                        aabb.max.y as f64 + spatial_offset.y,
                        0.0,
                    ))
                    .truncate();

                let cond = Box2D::new(
                    point(projected_min.x as f32, projected_min.y as f32),
                    point(projected_max.x as f32, projected_max.y as f32),
                )
                .intersects(screen_aabb);
                if !cond {
                    continue;
                }
                shape_data.path.iter().for_each(|path| match path {
                    PathEvent::Begin { at } => {
                        let projected = view_proj_matrix
                            .project_point3(DVec3::new(
                                at.x as f64 + spatial_offset.x,
                                at.y as f64 + spatial_offset.y,
                                0.0,
                            ))
                            .truncate();
                        pb.move_to((
                            0.5 * (1.0 + projected.x as f32) * Self::WIDTH as f32,
                            0.5 * (1.0 + projected.y as f32) * Self::HEIGHT as f32,
                        ));
                    }
                    PathEvent::Line { from: _, to } => {
                        let projected = view_proj_matrix
                            .project_point3(DVec3::new(
                                to.x as f64 + spatial_offset.x,
                                to.y as f64 + spatial_offset.y,
                                0.0,
                            ))
                            .truncate();
                        pb.line_to((
                            0.5 * (1.0 + projected.x as f32) * Self::WIDTH as f32,
                            0.5 * (1.0 + projected.y as f32) * Self::HEIGHT as f32,
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
                    .unwrap_or(&Color4f::new(0.0, 0.0, 0.0, 1.0))
                    .clone();
                paint.set_color(color.to_color());
                paint.set_anti_alias(true);

                if is_line {
                    paint.set_stroke_width(l_width);
                    paint.set_style(PaintStyle::Stroke);
                    canvas.draw_path(&path, &paint);
                    // pixmap.stroke_path(
                    //     &path,
                    //     &paint,
                    //     &Stroke {
                    //         width: l_width,
                    //         miter_limit: 1.0,
                    //         line_cap: Default::default(),
                    //         line_join: Default::default(),
                    //         dash: None,
                    //     },
                    //     Transform::default(),
                    //     None,
                    // )
                } else {
                    paint.set_style(PaintStyle::Fill);
                    canvas.draw_path(&path, &paint);
                    // pixmap.fill_path(
                    //     &path,
                    //     &paint,
                    //     tiny_skia::FillRule::Winding,
                    //     Transform::default(),
                    //     None,
                    // );
                }
            }
        });
    }
}

impl Renderer for CpuRenderer {
    type RAPI = CpuRendererApi;
    type OUTPUT = ();
    type INPUT<'a> = &'a Canvas;

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

    fn render2(&mut self, input: Self::INPUT<'_>) {
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
                    self.shapes_foreground
                        .push((key, spat_data, foreground_shapes));
                }
                RendererApiMsg::ClearGroups(keys) => {
                    self.shapes_background.retain(|s| !keys.contains(&s.0));
                    self.shapes_foreground.retain(|s| !keys.contains(&s.0));
                }
                RendererApiMsg::UpdateStyle(style_id, updater) => {
                    let mut style = RenderStyle::default();
                    updater(&mut style);
                    let fill_color = style.get_fill_color();

                    let fill_color =  Color4f::new(fill_color[0],
                                                   fill_color[1],
                                                   fill_color[2],
                                                   fill_color[3]);
                    self.styles_map.insert(style_id, fill_color);
                }
            }
        }

        self.norm_length = self.calc_normalized_vector_proj_length();

        let processor = |shapes: &[(String, SpatialData, Vec<(Box2D, ShapeData)>)],
                         canvas: &Canvas| {
            Self::process_shapes(
                shapes,
                canvas,
                &self.cs_offset,
                self.norm_length,
                &self.view_proj_matrix,
                &self.screen_aabb,
                &self.styles_map,
            );
        };
        let qq = Color4f::new(0.0, 0.0, 0.0, 1.0);
        let ground_color = self
            .styles_map
            .get(&self.ground_style)
            .unwrap_or(&qq);
        input.clear(ground_color.to_color());
        processor(&self.shapes_background, input);
        processor(&self.shapes_foreground, input);
        // let (mut pb, mut pa) = rayon::join(
        //     || {
        //         let ground_color = self
        //             .styles_map
        //             .get(&self.ground_style)
        //             .unwrap_or(&Color::BLACK);
        //
        //         // let mut pixmap_background = Pixmap::new(Self::WIDTH, Self::HEIGHT).unwrap();
        //         // input.fill(*ground_color);
        //         // processor(&self.shapes_background, &mut input);
        //         ()
        //     },
        //     || {
        //         // self.pixmap_fore.fill(Color::TRANSPARENT);
        //         processor(&self.shapes_foreground, input);
        //         ()
        //     },
        // );

        // Self::fast_blend2(&mut input, &mut pa);
        // Some(pb)
    }



    fn render(&mut self) -> Option<Self::OUTPUT> {
        None
    }

    fn api(&self) -> Arc<CpuRendererApi> {
        self.cpu_renderer_api.clone()
    }
}
