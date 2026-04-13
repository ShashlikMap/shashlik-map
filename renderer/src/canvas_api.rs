use crate::draw_commands::mesh2d_draw_command::{Mesh2dCommandBatch, Mesh2dDrawCommand};
use crate::draw_commands::mesh3d_draw_command::Mesh3dDrawCommand;
use crate::draw_commands::text_draw_command::TextDrawCommand;
use crate::draw_commands::{DrawCommand, DrawCommands, GeometryType, MeshVertex, PolylineOptions};
use crate::geometry_data::{ExtrudedPolygonData, GeometryData, ShapeData, SvgData, TextData};
use crate::mesh::mesh::{StyledRange, StyledRangeInfo};
use crate::modifier::render_modifier::SpatialData;
use crate::styles::render_style::RenderStyle;
use crate::styles::style_id::StyleId;
use crate::styles::style_store::StyleStore;
use crate::svg::svg_parser::svg_parse;
use crate::vertex_attrs::ShapeVertex;
use glam::{DVec3, Vec3};
use lyon::geom::euclid::{point2, Box2D, Point2D};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};
use lyon::path::{Path, Winding};
use std::collections::{BTreeMap, HashMap};
use std::mem;
use lyon::path::builder::BorderRadii;

#[derive(Clone)]
pub struct MeshInfo {
    pub instance_positions: Option<Vec<DVec3>>,
    pub size: Option<f32>,
    pub with_collision: bool,
    pub instance_key: String,
    pub double_style: bool,
}

pub struct CanvasApi {
    style_store: StyleStore,
    flushed: bool,
    draw_commands: Vec<Box<dyn DrawCommand>>,
    geometry: VertexBuffers<ShapeVertex, u32>,
    indices_by_layers: BTreeMap<i8, Vec<StyledRange>>,
    geometry3d: VertexBuffers<MeshVertex, u32>,
    text_vec: Vec<TextData>,
    mesh_info_cache: HashMap<&'static str, (VertexBuffers<ShapeVertex, u32>, MeshInfo)>,
    feature_layer_tag: Option<String>,
}

impl CanvasApi {

    pub fn new(style_store: StyleStore) -> CanvasApi {
        CanvasApi {
            style_store,
            flushed: false,
            draw_commands: Vec::new(),
            geometry: VertexBuffers::new(),
            indices_by_layers: BTreeMap::new(),
            geometry3d: VertexBuffers::new(),
            text_vec: Vec::new(),
            mesh_info_cache: HashMap::new(),
            feature_layer_tag: None,
        }
    }
    pub(crate) fn start_commands(&mut self) {
        self.flushed = false;
        self.feature_layer_tag = None;
        self.indices_by_layers.clear();
        self.geometry.clear();
        self.geometry3d.clear();
        self.text_vec.clear();

        // TODO Should be improved to per screen rather than per group
        self.mesh_info_cache
            .iter_mut()
            .for_each(|(_, (_, mesh_info))| {
                // keep only buffers, clean positions
                mesh_info.instance_positions = None;
                mesh_info.with_collision = false;
            })
    }

    pub fn set_feature_layer_tag(&mut self, tag: Option<String>) {
        self.feature_layer_tag = tag;
    }

    pub fn update_style<F: FnOnce(&mut RenderStyle)>(&mut self, style_id: &StyleId, updater: F) {
        self.style_store.update_style(style_id, updater);
    }

    pub fn geometry_data(&mut self, geometry_data: GeometryData) {
        match geometry_data {
            GeometryData::Shape(data) => {
                self.path(data);
            }
            GeometryData::ExtrudedPolygon(data) => {
                self.extruded_polygon(data);
            }
            GeometryData::Svg(data) => {
                self.svg(data);
            }
            GeometryData::Text(data) => {
                self.text(data);
            }
        }
    }

    fn prepare_mesh2d_command(&mut self) {
        let mesh = mem::replace(&mut self.geometry, VertexBuffers::new());
        if !mesh.vertices.is_empty() {
            let flatten_ranges = mem::take(&mut self.indices_by_layers)
                .into_values()
                .flatten()
                .collect();
            let mesh_info = MeshInfo {
                instance_positions: None,
                size: None,
                with_collision: false,
                instance_key: "".to_string(),
                double_style: true,
            };
            let batch = Mesh2dCommandBatch {
                mesh,
                layers_indices: flatten_ranges,
                mesh_info,
            };
            self.mesh2d_with_positions(vec![batch], false);
        }
    }

    fn prepare_mesh2d_screen_space_command(&mut self) {
        if self.mesh_info_cache.is_empty() {
            return;
        }
        let batches = self
            .mesh_info_cache
            .iter()
            .map(|(_, (mesh, positions))| (mesh.clone(), positions.clone()))
            .flat_map(|(mesh, mesh_info)| {
                // mesh_info_cache is always present but there is no positions then we skip command
                if mesh_info.instance_positions.is_none() {
                    return None;
                }
                let styled_range = StyledRange(0..mesh.indices.len(), StyledRangeInfo(0, ""));
                Some(Mesh2dCommandBatch {
                    mesh,
                    layers_indices: vec![styled_range],
                    mesh_info,
                })
            })
            .collect();

        self.mesh2d_with_positions(batches, true);
    }

    fn prepare_text_command(&mut self) {
        if self.text_vec.is_empty() {
            return;
        }
        self.draw_commands.push(Box::new(TextDrawCommand {
            data: mem::replace(&mut self.text_vec, Vec::new()),
        }));
    }

    fn mesh2d_with_positions(&mut self, batches: Vec<Mesh2dCommandBatch>, is_screen: bool) {
        self.draw_commands.push(Box::new(Mesh2dDrawCommand {
            batches,
            is_screen,
            feature_layer_tag: self.feature_layer_tag.clone(),
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
                let normal = Vec3::new(-(p2.y - p1.y), p2.x - p1.x, 0.0)
                    .normalize()
                    .into();

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

    fn prepare_mesh3d_command(&mut self) {
        let mesh = mem::replace(&mut self.geometry3d, VertexBuffers::new());
        if mesh.vertices.len() > 0 {
            self.draw_commands
                .push(Box::new(Mesh3dDrawCommand { mesh }));
        }
    }

    fn path(&mut self, data: ShapeData) {
        let geom_type = data.geometry_type;
        let style_index = self.style_store.get_index(&data.style_id);
        let initial_index = self.geometry.indices.len();
        match geom_type {
            GeometryType::Polyline(options) => {
                self.tessellate_stroke_path(&data.path, options, |vertex| ShapeVertex {
                    position: [vertex.position().x, vertex.position().y, 0.0f32],
                    normals: [vertex.normal().x, vertex.normal().y, 0.0],
                    uv_dist: [0.0, 0.0, vertex.advancement()],
                    style_index: style_index as u32,
                });
            }
            GeometryType::Polygon => {
                Self::tessellate_fill_path(&data.path, &mut self.geometry, |vertex| ShapeVertex {
                    position: [vertex.position().x, vertex.position().y, 0.0f32],
                    normals: [0.0, 0.0, 0.0],
                    uv_dist: [0.0, 0.0, 0.0], // fill doesn't have length
                    style_index: style_index as u32,
                });
            }
        }
        let last_index = self.geometry.indices.len();

        let ranges = self
            .indices_by_layers
            .entry(data.index_layer_level)
            .or_insert(Vec::new());
        if let Some(last) = ranges.last_mut()
            && last.0.end == initial_index
        {
            last.0.end = last_index;
        } else {
            ranges.push(StyledRange(
                initial_index..last_index,
                data.styled_range_info,
            ));
        }
    }

    fn svg(&mut self, data: SvgData) {
        self.mesh_info_cache
            .entry(data.icon.0)
            .and_modify(|(_, mesh_info)| {
                mesh_info
                    .instance_positions
                    .get_or_insert_default()
                    .push(data.position);
                mesh_info.with_collision = data.with_collision
            })
            .or_insert_with(|| {
                let style_index = self.style_store.get_index(&data.style_id);
                let mut mesh: VertexBuffers<ShapeVertex, u32> = VertexBuffers::new();
                let mut mesh_size = data.size;
                if let Some(svg_background) = data.background {
                    mesh_size += 2.0 * svg_background.padding;
                    let background_style_index = self.style_store.get_index(&svg_background.style_id);

                    let mut builder = Path::builder();
                    let half_size = svg_background.padding + data.size / 2.0;
                    let rect = Box2D::new(point2(-half_size, -half_size), point2(half_size, half_size));
                    builder.add_rounded_rectangle(&rect, &BorderRadii::new(10.0), Winding::Positive);
                    let path = builder.build();

                    Self::tessellate_fill_path(&path, &mut mesh, |vertex| ShapeVertex {
                        position: [vertex.position().x, vertex.position().y, 0.0f32],
                        normals: [0.0, 0.0, 0.0],
                        uv_dist: [0.0, 0.0, 0.0], // fill doesn't have length
                        style_index: background_style_index as u32,
                    });
                }

                svg_parse(data.icon.1, &mut mesh, data.size, style_index);

                (
                    mesh,
                    MeshInfo {
                        instance_positions: Some(vec![data.position]),
                        size: Some(mesh_size),
                        with_collision: data.with_collision,
                        instance_key: data.icon.0.to_string(),
                        double_style: false,
                    },
                )
            });
    }

    pub fn text(&mut self, data: TextData) {
        self.text_vec.push(data);
    }

    pub(crate) fn flush_commands(
        &mut self,
        key: String,
        spatial_data: SpatialData,
        spatial_tx: tokio::sync::broadcast::Sender<SpatialData>,
    ) -> DrawCommands {
        assert!(!self.flushed);
        self.flushed = true;

        self.prepare_mesh2d_command();
        self.prepare_mesh3d_command();
        self.prepare_mesh2d_screen_space_command();
        self.prepare_text_command();

        DrawCommands::new(
            key,
            spatial_data,
            spatial_tx,
            mem::take(&mut self.draw_commands),
        )
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

    fn tessellate_stroke_path<F>(&mut self, path: &Path, polyline_options: PolylineOptions, ctor: F)
    where
        F: Fn(StrokeVertex) -> ShapeVertex,
    {
        let mut tessellator = StrokeTessellator::new();
        {
            tessellator
                .tessellate_path(
                    path,
                    &StrokeOptions::default()
                        .with_line_width(polyline_options.width)
                        .with_line_cap(polyline_options.line_cap)
                        .with_line_join(polyline_options.line_join)
                        .with_tolerance(polyline_options.tolerance),
                    &mut BuffersBuilder::new(&mut self.geometry, ctor),
                )
                .unwrap();
        }
    }
}
