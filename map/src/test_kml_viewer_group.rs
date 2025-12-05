use cgmath::Vector3;
use geo_types::{Geometry, GeometryCollection, Point};
use kml::KmlReader;
use renderer::canvas_api::CanvasApi;
use renderer::geometry_data::{GeometryData, SvgData};
use renderer::render_group::RenderGroup;
use renderer::styles::style_id::StyleId;
use std::path::PathBuf;

pub struct TestKmlGroup {
    pub geom_coll: GeometryCollection<f64>,
}

impl TestKmlGroup {
    pub const CIRCLE_SVG: &'static [u8] = include_bytes!("../svg/just_circle.svg");

    pub fn new(path_buf: PathBuf, converter: Box<dyn Fn(&Point) -> Point>) -> TestKmlGroup {
        let mut kml_reader = KmlReader::<_, f64>::from_path(path_buf).unwrap();
        let kml_data = kml_reader.read().unwrap();
        let mut geom_coll: GeometryCollection<f64> = kml_data.try_into().unwrap();
        geom_coll.0.iter_mut().for_each(|geom| match geom {
            Geometry::Point(point) => {
                *point = converter(point);
            }
            _ => {}
        });
        TestKmlGroup { geom_coll }
    }
}

impl RenderGroup for TestKmlGroup {
    fn content(&mut self, canvas: &mut CanvasApi) {
        let mut geometry_data = vec![];
        self.geom_coll.0.iter().for_each(|geom| match geom {
            Geometry::Point(point) => {
                geometry_data.push(GeometryData::Svg(SvgData {
                    icon: ("kml", Self::CIRCLE_SVG),
                    position: Vector3::new(point.x(), point.y(), 0.0).cast().unwrap(),
                    size: 20.0,
                    style_id: StyleId("kml_dots"),
                    with_collision: false
                }));
            }
            _ => {}
        });

        geometry_data.into_iter().for_each(|data| {
            canvas.geometry_data(data);
        });
    }
}
