use crate::tiles::mesh_loader::MeshLoader;
use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::TilesProvider;
use futures::Stream;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use geo_types::Rect;
use renderer::draw_commands::{GeometryType, PolylineOptions};
use renderer::geometry_data::{GeometryData, Mesh3d, ShapeData};
use renderer::styles::style_id::StyleId;
use std::collections::HashSet;

pub struct StubTilesProvider {
    sender: Option<UnboundedSender<(Option<TileData>, HashSet<String>)>>,
}

impl StubTilesProvider {
    pub fn new() -> Self {
        Self { sender: None }
    }
}

impl TilesProvider for StubTilesProvider {

    fn load(&mut self, _area: Rect, _zoom_level: i32) {
        let polygon_path = MeshLoader::load_test_polygon_path();
        let line_path = MeshLoader::load_test_line_path();
        let line_path2 = MeshLoader::load_test_line2_path();
        let obj_mesh = MeshLoader::load_from_obj(include_bytes!("../../cube.obj"));
        let options = PolylineOptions {
            width: 0.7,
        };
        let poly_line_geom_type = GeometryType::Polyline(options);
        let tile_data = TileData {
            key: "".to_string(),
            position: [1.0, 0.0, 0.0].into(),
            size: (0.0, 0.0),
            geometry_data: vec![
                GeometryData::Shape(ShapeData {
                    path: line_path,
                    geometry_type: poly_line_geom_type,
                    style_id: StyleId("land"),
                    layer_level: 0,
                    is_screen: false,
                }),
                GeometryData::Shape(ShapeData {
                    path: line_path2,
                    geometry_type: poly_line_geom_type,
                    style_id: StyleId("land"),
                    layer_level: 0,
                    is_screen: false,
                }),
                GeometryData::Shape(ShapeData {
                    path: polygon_path,
                    geometry_type: GeometryType::Polygon,
                    style_id: StyleId("land"),
                    layer_level: 0,
                    is_screen: false,
                }),
                GeometryData::Mesh3d(Mesh3d {
                    mesh_data: obj_mesh,
                }),
            ],
        };

        if let Some(sender) = &self.sender {
            sender.unbounded_send((Some(tile_data), HashSet::new())).unwrap();
        }
    }

    fn tiles(&mut self) -> impl Stream<Item = (Option<TileData>, HashSet<String>)> + Send + 'static {
        let (sender, receiver) = unbounded();
        self.sender = Some(sender);

        receiver
    }
}
