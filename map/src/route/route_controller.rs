use crate::route::RouteCosting;
use crate::route::route_group::RouteGroup;
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
#[cfg(target_os = "android")]
extern crate valhalla_client_android as valhalla_client;

pub struct RouteController {
    current_lon_lat: Option<(f64, f64)>,
    valhalla: Arc<Valhalla>
}

impl RouteController {
    pub fn new() -> RouteController {
        RouteController {
            current_lon_lat: None,
            valhalla: Arc::new(Valhalla::default())
        }
    }
    pub fn set_current_lon_lat(&mut self, lon_lat: (f64, f64)) {
        self.current_lon_lat = Some(lon_lat);
    }

    pub fn calc_route(
        &self,
        to_lon_lat: (f64, f64),
        route_costing: RouteCosting,
        converter: Box<dyn (Fn(&Point) -> Point) + Send>,
        api: Arc<RendererApi>,
    ) {
        if let Some((lon, lat)) = self.current_lon_lat {
            let valhalla = Arc::clone(&self.valhalla);
            spawn(move || {
                let source_loc = Location::new(lon as f32, lat as f32);
                let destination_loc = Location::new(to_lon_lat.0 as f32, to_lon_lat.1 as f32);
                let costing = match route_costing {
                    RouteCosting::Pedestrian => Costing::Pedestrian(Default::default()),
                    RouteCosting::Motorbike => Costing::Motorcycle(Default::default()),
                    RouteCosting::Auto => Costing::Auto(Default::default()),
                };
                let manifest = Manifest::builder()
                    .locations([source_loc, destination_loc])
                    .directions_type(DirectionsType::None)
                    .costing(costing);

                Self::clear_routes_internal(api.clone());
                match valhalla.route(manifest) {
                    Ok(trip) => {
                        // println!("Route calculated: {:?}", trip);
                        println!("Route calculated!");
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
                            api.add_render_group("route".to_string(), spatial_data, route);
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

    pub fn clear_routes(&self, api: Arc<RendererApi>) {
        Self::clear_routes_internal(api);
    }

    fn clear_routes_internal(api: Arc<RendererApi>) {
        api.clear_render_groups(HashSet::from_iter(vec!["route".to_string()]));
    }
}
