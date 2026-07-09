use crate::route::{RouteCosting};
use crate::route::route_group::RouteGroup;
use geo_types::{Point, point};
use log::{error};
use renderer_common::render_modifier::SpatialData;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::Duration;
use valhalla_client::blocking::Valhalla;
use valhalla_client::costing::Costing;
use valhalla_client::route::{DirectionsType, Location, Manifest};
use renderer_common::{RendererApi};

#[cfg(target_os = "android")]
extern crate valhalla_client_android as valhalla_client;

pub struct RouteController<RAPI: RendererApi + 'static> {
    api: Arc<RAPI>,
    current_lon_lat: Option<(f64, f64)>,
    valhalla: Arc<Valhalla>
}

impl <RAPI: RendererApi + 'static> RouteController<RAPI> {
    pub fn new(api: Arc<RAPI>) -> RouteController<RAPI> {
        let mut route_controller = RouteController {
            api,
            current_lon_lat: None,
            valhalla: Arc::new(Valhalla::default())
        };
        route_controller.warm_up();
        route_controller
    }
    pub fn set_current_lon_lat(&mut self, lon_lat: (f64, f64)) {
        self.current_lon_lat = Some(lon_lat);
    }

    fn warm_up(&mut self) {
        // Route rendering uses indirect drawing feature
        // On Linux target(Rasp4 with Vulkan) indirect pipeline may take up to 2-3 seconds
        // It's not clear if it's a bug or not.
        // The below provides a dummy route to warm up a pipeline.
        // Also, it's been found that indirect drawing doesn't seem to be working on iOS simulator,
        // so it's better to isolate warming up only for linux for now
        #[cfg(target_os = "linux")] {
            let route: Vec<Point> = vec![point!(x:0.0, y:0.0), point!(x: 1.0, y:0.0)];
            let route = Box::new(RouteGroup::new(route, RouteCosting::Auto));
            let spatial_data = SpatialData::transform(route.first_route_point());
            self.api.add_render_group("route".to_string(), spatial_data, route);
        }
    }

    pub fn calc_route(
        &self,
        to_lon_lat: (f64, f64),
        route_costing: RouteCosting,
        converter: Box<dyn (Fn(&Point) -> Point) + Send>,
    ) {
        if let Some((lon, lat)) = self.current_lon_lat {
            let valhalla = Arc::clone(&self.valhalla);
            let api = Arc::clone(&self.api);
            spawn(move || {
                let source_loc = Location::new(lon as f32, lat as f32);
                let destination_loc = Location::new(to_lon_lat.0 as f32, to_lon_lat.1 as f32);
                let costing = match route_costing {
                    RouteCosting::Pedestrian => Costing::Pedestrian(Default::default()),
                    RouteCosting::Motorbike => Costing::Motorcycle(Default::default()),
                    RouteCosting::Auto => Costing::Auto(Default::default()),
                };

                Self::clear_routes_internal(api.clone());

                let max_attempts = 2;
                for attempt in 1..=max_attempts {
                    let manifest = Manifest::builder()
                        .locations([source_loc.clone(), destination_loc.clone()])
                        .directions_type(DirectionsType::None)
                        .costing(costing.clone());

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
                                let route: Vec<Point> = route.iter().map(|p| converter(p)).collect();

                                let route = Box::new(RouteGroup::new(route, route_costing));
                                let spatial_data = SpatialData::transform(route.first_route_point());
                                api.add_render_group("route".to_string(), spatial_data, route);
                            } else {
                                error!("No legs found in route!");
                            }
                            break;
                        }
                        Err(err) => {
                            if attempt < max_attempts {
                                error!("Attempt {} failed: {:?}. Retrying...", attempt, err);
                                sleep(Duration::from_secs(1));
                            } else {
                                error!("Error calculating route after {} attempts: {:?}", max_attempts, err);
                            }
                        }
                    }
                }
            });
        }
    }

    pub fn clear_routes(&self, api: Arc<RAPI>) {
        Self::clear_routes_internal(api);
    }

    fn clear_routes_internal(api: Arc<RAPI>) {
        api.clear_render_groups(HashSet::from_iter(vec!["route".to_string()]));
    }
}
