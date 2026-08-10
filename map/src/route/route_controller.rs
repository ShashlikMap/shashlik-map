use crate::route::RouteCosting;
use crate::route::route_group::RouteGroup;
use geo_types::{Point, point};
use log::error;
use renderer_common::RendererApi;
use renderer_common::render_modifier::SpatialData;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread::{sleep, spawn};
use std::time::Duration;
use valhalla_client::blocking::Valhalla;
use valhalla_client::costing::Costing;
use valhalla_client::{Error};
use valhalla_client::route::{DirectionsType, Location, Manifest, Trip};

pub struct RouteController<RAPI: RendererApi + 'static> {
    api: Arc<RAPI>,
    current_lon_lat: Option<(f64, f64)>,
    valhalla: Arc<Valhalla>,
    active_routes: Arc<AtomicU8>
}

impl<RAPI: RendererApi + 'static> RouteController<RAPI> {
    pub fn new(api: Arc<RAPI>) -> RouteController<RAPI> {
        let mut route_controller = RouteController {
            api,
            current_lon_lat: None,
            valhalla: Arc::new(Valhalla::default()),
            active_routes: Arc::new(AtomicU8::new(0))
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
        #[cfg(target_os = "linux")]
        {
            let route: Vec<Point> = vec![point!(x:0.0, y:0.0), point!(x: 1.0, y:0.0)];
            let route = Box::new(RouteGroup::new(route, RouteCosting::Auto));
            let spatial_data = SpatialData::transform(route.first_route_point());
            self.api
                .add_render_group("route".to_string(), spatial_data, route);
        }
    }

    pub fn calc_route(
        &mut self,
        to_lon_lat: (f64, f64),
        route_costing: RouteCosting,
        converter: Box<dyn (Fn(&Point) -> Point) + Send>,
    ) {
        self.clear_routes(Arc::clone(&self.api));
        if let Some((lon, lat)) = self.current_lon_lat {
            let valhalla = Arc::clone(&self.valhalla);
            let api = Arc::clone(&self.api);
            let active_routes = Arc::clone(&self.active_routes);
            spawn(move || {
                let source_loc = Location::new(lon as f32, lat as f32);
                let destination_loc = Location::new(to_lon_lat.0 as f32, to_lon_lat.1 as f32);
                let costing = match route_costing {
                    RouteCosting::Pedestrian => Costing::Pedestrian(Default::default()),
                    RouteCosting::Motorbike => Costing::Motorcycle(Default::default()),
                    RouteCosting::Auto => Costing::Auto(Default::default()),
                };

                let max_attempts = 2;
                for attempt in 1..=max_attempts {
                    let manifest = Manifest::builder()
                        .locations([source_loc.clone(), destination_loc.clone()])
                        .alternates(2)
                        .directions_type(DirectionsType::None)
                        .costing(costing.clone());

                    match valhalla.route_with_alternatives(manifest) {
                        Ok(trips) => {
                            let alternates = trips.1.len() as u8;
                            Self::handle_trips(trips, |index, trip: &Trip| {
                                if let Some(leg) = trip.legs.first() {
                                    let route: Vec<Point> = leg
                                        .shape
                                        .iter()
                                        .map(|p| {
                                            point! { x: p.lon, y: p.lat }
                                        })
                                        .collect();
                                    let route: Vec<Point> =
                                        route.iter().map(|p| converter(p)).collect();
                                    let route = Box::new(RouteGroup::new(
                                        route,
                                        index < alternates,
                                        route_costing.clone(),
                                    ));
                                    let spatial_data =
                                        SpatialData::transform(route.first_route_point());

                                    active_routes.store(index, Ordering::Relaxed);
                                    api.add_render_group(
                                        Self::create_route_id(index),
                                        spatial_data,
                                        route,
                                    );
                                } else {
                                    error!("No legs found in route!");
                                }
                            });
                            break;
                        }
                        Err(err) => {
                            if attempt < max_attempts {
                                error!("Attempt {} failed: {:?}. Retrying...", attempt, err);
                                sleep(Duration::from_secs(1));
                            } else {
                                error!(
                                    "Error calculating route after {} attempts: {:?}",
                                    max_attempts, err
                                );
                            }
                        }
                    }
                }
            });
        }
    }

    pub fn handle_trips(trips: (Trip, Vec<Trip>), action: impl Fn(u8, &Trip) -> ()) {
        let main_trip = &trips.0;
        let alternates: Vec<Trip> = trips.1.clone();
        println!("Route calculated!, alternates = {:?}", alternates.len());
        vec![main_trip]
            .into_iter()
            .chain(alternates.iter())
            .rev()
            .enumerate()
            .for_each(|(index, trip)| {
                action(index as u8, trip);
            });
    }

    pub fn clear_routes(&mut self, api: Arc<RAPI>) {
        let active_routes = self.active_routes.swap(0, Ordering::Relaxed);
        api.clear_render_groups(HashSet::from_iter((0..=active_routes).map(Self::create_route_id)));
    }

    fn create_route_id(index: u8) -> String {
        format!("route{index}").to_string()
    }
}
