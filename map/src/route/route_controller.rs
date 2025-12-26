use crate::route::RouteCosting;
use crate::route::route_group::RouteGroup;
use cgmath::Vector3;
use geo_types::{Point, point};
use log::error;
use renderer::modifier::render_modifier::SpatialData;
use renderer::renderer_api::RendererApi;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread::spawn;
use valhalla_client::blocking::Valhalla;
use valhalla_client::costing::Costing;
use valhalla_client::route::{DirectionsType, Location, Manifest};

pub struct RouteController {
    current_lat_lon: Option<(f64, f64)>,
}

impl RouteController {
    pub fn new() -> RouteController {
        RouteController {
            current_lat_lon: None,
        }
    }
    pub fn set_current_lat_lon(&mut self, lat_lon: (f64, f64)) {
        self.current_lat_lon = Some(lat_lon);
    }

    pub fn calc_route(
        &self,
        to_lat_lon: (f64, f64),
        route_costing: RouteCosting,
        converter: Box<dyn (Fn(&Point) -> Point) + Send>,
        api: Arc<RendererApi>,
    ) {
        if let Some((lat, lon)) = self.current_lat_lon {
            spawn(move || {
                let valhalla = Valhalla::default();

                let source_loc = Location::new(lon as f32, lat as f32);
                let destination_loc = Location::new(to_lat_lon.0 as f32, to_lat_lon.1 as f32);
                let costing = match route_costing {
                    RouteCosting::Pedestrian => Costing::Pedestrian(Default::default()),
                    RouteCosting::Motorbike => Costing::Motorcycle(Default::default()),
                };
                let manifest = Manifest::builder()
                    .locations([source_loc, destination_loc])
                    .directions_type(DirectionsType::None)
                    .costing(costing);

                api.clear_render_groups(HashSet::from_iter(vec!["route".to_string()]));
                match valhalla.route(manifest) {
                    Ok(trip) => {
                        println!("Route calculated: {:?}", trip);
                        if let Some(leg) = trip.legs.first() {
                            let route: Vec<Point> = leg
                                .shape
                                .iter()
                                .map(|p| {
                                    point! { x: p.lon, y: p.lat }
                                })
                                .collect();

                            let route = Box::new(RouteGroup::new(route, route_costing, converter));
                            let spatial_data = SpatialData::transform(route.first_route_point());
                            api.add_render_group("route".to_string(), 1, spatial_data, route);
                        } else {
                            error!("No legs found in route!");
                        }
                    }
                    Err(err) => {
                        error!("Error calculating route: {:?}", err);
                    }
                }
            });
        }
    }
}
