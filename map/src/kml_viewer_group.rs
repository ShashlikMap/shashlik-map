use cgmath::Vector3;
use geo_types::{Geometry, GeometryCollection, Point};
use kml::KmlReader;
use log::error;
use renderer::canvas_api::CanvasApi;
use renderer::geometry_data::{GeometryData, SvgData};
use renderer::render_group::RenderGroup;
use renderer::styles::style_id::StyleId;
use std::path::PathBuf;

pub struct KmlGroup {
    pub collection: GeometryCollection<f64>,
}

impl KmlGroup {
    pub const CIRCLE_SVG: &'static [u8] = include_bytes!("../svg/just_circle.svg");

    pub fn new(path_buf: PathBuf, converter: Box<dyn Fn(&Point) -> Point>) -> KmlGroup {
        let mut kml_reader = KmlReader::<_, f64>::from_path(path_buf).unwrap();
        let kml_data = kml_reader.read().unwrap();

        let mut collection: GeometryCollection<f64> = kml_data.try_into().unwrap();
        Self::convert_collection(&mut collection, &converter);
        KmlGroup { collection }
    }

    fn convert_collection(
        collection: &mut GeometryCollection<f64>,
        converter: &Box<dyn Fn(&Point) -> Point>,
    ) {
        collection.0.iter_mut().for_each(|geom| match geom {
            Geometry::Point(point) => {
                *point = converter(point);
            }
            Geometry::GeometryCollection(collection) => {
                Self::convert_collection(collection, converter);
            }
            _ => {
                println!("WARNING: KML contains invalid geometry type {:?}", geom);
            }
        });
    }

    fn populate_geometry(
        collection: &GeometryCollection<f64>,
        geometry_data: &mut Vec<GeometryData>,
    ) {
        collection.iter().for_each(|geom| match geom {
            Geometry::Point(point) => {
                geometry_data.push(GeometryData::Svg(SvgData {
                    icon: ("kml", Self::CIRCLE_SVG),
                    position: Vector3::new(point.x(), point.y(), 0.0).cast().unwrap(),
                    size: 20.0,
                    style_id: StyleId("kml_dots"),
                    with_collision: false,
                }));
            }
            Geometry::GeometryCollection(collection) => {
                Self::populate_geometry(collection, geometry_data);
            }
            _ => {
                error!("collection contains invalid geometry type {:?}", geom);
            }
        });
    }
}

impl RenderGroup for KmlGroup {
    fn content(&mut self, canvas: &mut CanvasApi) {
        let mut geometry_data = vec![];
        Self::populate_geometry(&self.collection, &mut geometry_data);
        geometry_data.into_iter().for_each(|data| {
            canvas.geometry_data(data);
        });
    }
}
