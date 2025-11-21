use crate::draw_commands::mesh2d_draw_command::Mesh2dDrawCommand;
use crate::draw_commands::mesh3d_draw_command::Mesh3dDrawCommand;
use crate::draw_commands::text_draw_command::TextDrawCommand;
use crate::draw_commands::text_draw_command2::TextDrawCommand2;
use crate::draw_commands::{DrawCommand, DrawCommands, GeometryType, MeshVertex};
use crate::geometry_data::{ExtrudedPolygonData, GeometryData, ShapeData, SvgData, TextData};
use crate::modifier::render_modifier::SpatialData;
use crate::styles::render_style::RenderStyle;
use crate::styles::style_id::StyleId;
use crate::styles::style_store::StyleStore;
use crate::svg::svg_parser::svg_parse;
use crate::text::glyph_tesselator::GlyphTesselator;
use crate::text::text_renderer::GlyphData;
use crate::vertex_attrs::ShapeVertex;
use cgmath::{Deg, Matrix4, Vector2, Vector3, Vector4};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};
use lyon::path::Path;
use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{ttf_parser, UnicodeBuffer};
use std::collections::{BTreeMap, HashMap};
use std::mem;
use std::ops::Range;

#[derive(Clone)]
pub struct ScreenPaths {
    pub positions: Vec<Vector3<f64>>,
    pub with_collision: bool,
}

pub struct CanvasApi {
    style_store: StyleStore,
    flushed: bool,
    draw_commands: Vec<Box<dyn DrawCommand>>,
    geometry: VertexBuffers<ShapeVertex, u32>,
    indices_by_layers: BTreeMap<i8, Vec<Range<usize>>>,
    real_layer: usize,
    geometry3d: VertexBuffers<MeshVertex, u32>,
    text_vec: Vec<TextData>,
    screen_path_cache: HashMap<&'static str, (VertexBuffers<ShapeVertex, u32>, ScreenPaths)>,
}

impl CanvasApi {
    pub fn new(style_store: StyleStore) -> CanvasApi {
        CanvasApi {
            style_store,
            flushed: false,
            draw_commands: Vec::new(),
            geometry: VertexBuffers::new(),
            indices_by_layers: BTreeMap::new(),
            real_layer: 0,
            geometry3d: VertexBuffers::new(),
            text_vec: Vec::new(),
            screen_path_cache: HashMap::new(),
        }
    }
    pub(crate) fn begin_shape(&mut self, real_layer: usize) {
        self.flushed = false;
        self.indices_by_layers.clear();
        self.geometry.clear();
        self.geometry3d.clear();
        self.text_vec.clear();
        self.real_layer = real_layer;

        // TODO Should be improved to per screen rather than per group
        self.screen_path_cache
            .iter_mut()
            .for_each(|(_, (_, screen_paths))| {
                // keep only buffers, clean positions
                screen_paths.positions.clear();
                screen_paths.with_collision = false
            })
    }
    pub fn update_style<F: FnOnce(&mut RenderStyle)>(&mut self, style_id: &StyleId, updater: F) {
        self.style_store.update_style(style_id, updater);
    }
    pub(crate) fn draw_commands(
        &mut self,
        key: String,
        spatial_data: SpatialData,
        spatial_tx: tokio::sync::broadcast::Sender<SpatialData>,
    ) -> DrawCommands {
        DrawCommands::new(
            key,
            spatial_data,
            spatial_tx,
            mem::take(&mut self.draw_commands),
        )
    }

    pub fn geometry_data(&mut self, geometry_data: GeometryData) {
        match geometry_data {
            GeometryData::Shape(data) => {
                self.path(data);
            }
            GeometryData::Mesh3d(data) => {
                self.mesh3d(data.mesh_data);
            }
            GeometryData::ExtrudedPolygon(data) => {
                self.extruded_polygon(data);
            }
            GeometryData::Svg(data) => {
                self.svg(&data);
            }
            GeometryData::Text(data) => {
                self.text(data);
            }
        }
    }

    fn mesh2d(&mut self, is_screen: bool) {
        let mesh = mem::replace(&mut self.geometry, VertexBuffers::new());
        if !mesh.vertices.is_empty() {
            let flatten_ranges = mem::take(&mut self.indices_by_layers).into_values().flatten().collect();
            let screen_paths = ScreenPaths {
                positions: vec![Vector3::new(0.0, 0.0, 0.0)],
                with_collision: false,
            };
            self.mesh2d_with_positions(mesh, flatten_ranges, screen_paths, is_screen);
        }
    }

    fn mesh2d_with_positions(
        &mut self,
        mesh: VertexBuffers<ShapeVertex, u32>,
        layers_indices: Vec<Range<usize>>,
        screen_paths: ScreenPaths,
        is_screen: bool,
    ) {
        self.draw_commands.push(Box::new(Mesh2dDrawCommand {
            mesh,
            real_layer: self.real_layer,
            layers_indices,
            screen_paths,
            is_screen,
        }));
    }

    pub fn extruded_polygon(&mut self, data: ExtrudedPolygonData) {
        let path = &data.path;
        let height = data.height;
        let mut geometry_buffer: VertexBuffers<MeshVertex, u32> = VertexBuffers::new();
        Self::tessellate_fill_path(path, &mut geometry_buffer, |vertex: FillVertex| {
            MeshVertex {
                position: [vertex.position().x, vertex.position().y, height],
                normals: [0.0, 0.0, 1.0],
            }
        });

        for path_event in path.iter() {
            let fi = geometry_buffer.vertices.len();
            if path_event.is_edge() {
                let p1 = path_event.from();
                let p2 = path_event.to();
                let normal = Vector3::new(-(p2.y - p1.y), p2.x - p1.x, 0.0).into();

                geometry_buffer.vertices.push(MeshVertex {
                    position: [p1.x, p1.y, 0.0],
                    normals: normal,
                });
                geometry_buffer.vertices.push(MeshVertex {
                    position: [p2.x, p2.y, 0.0],
                    normals: normal,
                });

                geometry_buffer.vertices.push(MeshVertex {
                    position: [p1.x, p1.y, height],
                    normals: normal,
                });

                geometry_buffer.vertices.push(MeshVertex {
                    position: [p2.x, p2.y, height],
                    normals: normal,
                });

                geometry_buffer.indices.push((fi + 0) as u32);
                geometry_buffer.indices.push((fi + 2) as u32);
                geometry_buffer.indices.push((fi + 3) as u32);

                geometry_buffer.indices.push((fi + 1) as u32);
                geometry_buffer.indices.push((fi + 0) as u32);
                geometry_buffer.indices.push((fi + 3) as u32);
            }
        }

        let fi = self.geometry3d.vertices.len();

        self.geometry3d.vertices.extend(geometry_buffer.vertices);
        self.geometry3d.indices.extend(
            geometry_buffer
                .indices
                .iter()
                .map(|i| *i + fi as u32)
                .collect::<Vec<u32>>(),
        );
    }

    pub fn mesh3d(&mut self, mesh: VertexBuffers<MeshVertex, u32>) {
        self.draw_commands
            .push(Box::new(Mesh3dDrawCommand { mesh }));
    }

    pub fn path(&mut self, data: ShapeData) {
        let geom_type = data.geometry_type;
        let style_index = self.style_store.get_index(&data.style_id);
        let initial_index = self.geometry.indices.len();
        match geom_type {
            GeometryType::Polyline(options) => {
                self.tessellate_stroke_path(&data.path, options.width, |vertex| ShapeVertex {
                    position: [vertex.position().x, vertex.position().y, 0.0f32],
                    normals: [vertex.normal().x, vertex.normal().y, 0.0],
                    dist: vertex.advancement(),
                    style_index: style_index as u32,
                });
            }
            GeometryType::Polygon => {
                Self::tessellate_fill_path(&data.path, &mut self.geometry, |vertex| ShapeVertex {
                    position: [vertex.position().x, vertex.position().y, 0.0f32],
                    normals: [0.0, 0.0, 0.0],
                    dist: 0.0, // fill doesn't have length
                    style_index: style_index as u32,
                });
            }
        }
        let last_index = self.geometry.indices.len();

        let ranges = self
            .indices_by_layers
            .entry(data.index_layer_level)
            .or_insert(Vec::new());
        if let Some(last) = ranges.last_mut() {
            if last.end == initial_index {
                last.end = last_index;
            } else {
                ranges.push(initial_index..last_index);
            }
        } else {
            ranges.push(initial_index..last_index);
        }

        // TODO It should aggregate geometry for "screen" type layers
        if data.is_screen {
            self.mesh2d(true);
        }
    }

    pub fn svg(&mut self, data: &SvgData) {
        self.screen_path_cache
            .entry(data.icon.0)
            .and_modify(|(_, screen_paths)| {
                screen_paths.positions.push(data.position);
                screen_paths.with_collision = data.with_collision
            })
            .or_insert_with(|| {
                let style_index = self.style_store.get_index(&data.style_id);
                let mesh = svg_parse(data.icon.1, data.size, style_index);
                (
                    mesh,
                    ScreenPaths {
                        positions: vec![data.position],
                        with_collision: data.with_collision,
                    },
                )
            });
    }

    pub fn rb_text_experiment(&mut self, str: &str, x_off: f32) {
        let face = ttf_parser::Face::parse(include_bytes!("font.ttf"), 0).unwrap();
        let face = rustybuzz::Face::from_face(face);

        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(str);
        buffer.guess_segment_properties();

        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
        let mut pos = x_off;

        let mut glyphs = vec![];

        let mut rotation = 0.0f32;
        let mut rotation2 = 0.0f32;
        for index in 0..glyph_buffer.len() {
            let position = glyph_buffer.glyph_positions()[index];
            let glyph_info = glyph_buffer.glyph_infos()[index];
            let mut path_builder = GlyphTesselator::new(0.01);
            face.outline_glyph(GlyphId(glyph_info.glyph_id as u16), &mut path_builder);

            let rotation_matrix = Matrix4::<f32>::from_angle_z(Deg(rotation));
            let p = rotation_matrix * Vector4::new(pos, 0.0, 0.0, 1.0);

            glyphs.push(GlyphData {
                glyph_id: GlyphId(glyph_info.glyph_id as u16),
                rotation: rotation2,
                offset: Vector2::new(p.x, p.y),
            });
            pos += position.x_advance as f32 * 0.01;
            rotation += 6.42857142865f32;
            rotation2 += 12.8571428571f32;
        }

        self.draw_commands.push(Box::new(TextDrawCommand2 {
            glyphs,
        }));
    }

    pub fn text(&mut self, data: TextData) {
        self.text_vec.push(data);
    }

    pub(crate) fn flush(&mut self) {
        assert!(!self.flushed);
        self.flushed = true;

        self.mesh2d(false);

        let mesh3d = mem::replace(&mut self.geometry3d, VertexBuffers::new());
        if mesh3d.vertices.len() > 0 {
            self.mesh3d(mesh3d);
        }

        if !self.screen_path_cache.is_empty() {
            let data: Vec<_> = self
                .screen_path_cache
                .iter()
                .map(|(_, (mesh, positions))| (mesh.clone(), positions.clone()))
                .collect();
            for (mesh, screen_paths) in data {
                if !screen_paths.positions.is_empty() {
                    let layers_indices = vec![0..mesh.indices.len()];
                    self.mesh2d_with_positions(mesh, layers_indices, screen_paths, true);
                }
            }
        }

        if !self.text_vec.is_empty() {
            self.draw_commands.push(Box::new(TextDrawCommand {
                data: mem::replace(&mut self.text_vec, Vec::new()),
            }));
        }
    }

    fn tessellate_fill_path<F, VT>(path: &Path, geometry: &mut VertexBuffers<VT, u32>, ctor: F)
    where
        F: Fn(FillVertex) -> VT,
    {
        let mut tessellator = FillTessellator::new();
        {
            tessellator
                .tessellate_path(
                    path,
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(geometry, ctor),
                )
                .unwrap();
        }
    }

    fn tessellate_stroke_path<F>(&mut self, path: &Path, width: f32, ctor: F)
    where
        F: Fn(StrokeVertex) -> ShapeVertex,
    {
        let mut tessellator = StrokeTessellator::new();
        {
            tessellator
                .tessellate_path(
                    path,
                    &StrokeOptions::default().with_line_width(width),
                    &mut BuffersBuilder::new(&mut self.geometry, ctor),
                )
                .unwrap();
        }
    }
}
