use crate::route::RouteCosting;
use geo_types::{coord, Point, LineString};
use log::error;
use renderer_common::RendererApi;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread::{sleep, spawn};
use std::time::Duration;
use geo::{Distance, Euclidean};
use glam::DVec3;
use reqwest::Url;
use valhalla_client::blocking::Valhalla;
use valhalla_client::costing::Costing;
use valhalla_client::route::{DirectionsType, Location, Manifest, Trip};
use renderer_common::geometry_data::{GeometryData, IconData, IconShapeData, IconType};
use renderer_common::geometry_render_group::GeometryRenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::style_id::StyleId;
use crate::CoordConverter;
use crate::overlay::overlay_shape_group::{OverlayShapeGroup};
use crate::overlay::ShapeType;

struct RouteData {
    id: String,
    route: LineString,
    costing: RouteCosting,
    style_id: StyleId,
}

pub struct RouteController<RAPI: RendererApi + 'static> {
    api: Arc<RAPI>,
    current_lon_lat: Option<(f64, f64)>,
    valhalla: Arc<Valhalla>,
    active_routes: Arc<AtomicU8>,
    full_routes: Arc<RwLock<Vec<RouteData>>>,
    active_routes_ids: Vec<String>,
    dpi_scale: f32,
}

impl<RAPI: RendererApi + 'static> RouteController<RAPI> {
    const PIN_SVG: &'static [u8] = include_bytes!("../../svg/location_pin.svg");

    pub fn new(url: Option<Url>, api: Arc<RAPI>, dpi_scale: f32) -> RouteController<RAPI> {
        let valhalla = url.map_or_else(Valhalla::default, Valhalla::new);
        let mut route_controller = RouteController {
            api,
            current_lon_lat: None,
            valhalla: Arc::new(valhalla),
            active_routes: Arc::new(AtomicU8::new(0)),
            full_routes: Arc::new(RwLock::new(vec![])),
            active_routes_ids: Vec::new(),
            dpi_scale,
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
            let route = OverlayShapeGroup::new(route, "route_layer".to_string(), StyleId::new("route"), ShapeType::Line(None), None, None, None);
            self.api
                .add_render_group("route".to_string(), route.spatial_data(), route);
        }
    }

    pub fn calc_route(
        &mut self,
        to_lon_lat: (f64, f64),
        route_costing: RouteCosting,
        converter: CoordConverter,
    ) {
        self.clear_routes(Arc::clone(&self.api));
        if let Some((lon, lat)) = self.current_lon_lat {
            let valhalla = Arc::clone(&self.valhalla);
            let api = Arc::clone(&self.api);
            Self::destination_pin(api.clone(), self.dpi_scale, None);
            let active_routes = Arc::clone(&self.active_routes);
            let dpi_scale = self.dpi_scale;
            let full_routes = Arc::clone(&self.full_routes);
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
                            // just apply all routes, even though some might not be created(rare)
                            active_routes.store(alternates + 1, Ordering::Relaxed);
                            let mut full_routes = full_routes.write().unwrap();
                            full_routes.clear();
                            Self::handle_trips(trips, |index, trip: &Trip| {
                                if let Some(leg) = trip.legs.first() {
                                    let route_id = Self::create_route_id(index);
                                    let route: Vec<Point> = leg
                                        .shape
                                        .iter()
                                        .map(|p| {
                                            converter(&p.into())
                                        })
                                        .collect();

                                    let shape_type: ShapeType = if matches!(route_costing, RouteCosting::Pedestrian) {
                                        ShapeType::DottedLine
                                    } else {
                                        ShapeType::Line(None)
                                    };
                                    let orig_style_id = Self::get_style_for_route(index < alternates, &route_costing);
                                    let style_id = StyleId(route_id.clone().into());
                                    api.copy_style(orig_style_id, style_id.clone());

                                    full_routes.push(RouteData {
                                        id: route_id.clone(),
                                        route: route.clone().into(),
                                        costing: route_costing.clone(),
                                        style_id: style_id.clone(),
                                    });

                                    let route_group = OverlayShapeGroup::new(
                                        route,
                                        "route_layer".to_string(),
                                        style_id,
                                        shape_type,
                                        None, None, None
                                    );

                                    api.add_render_group(
                                        route_id,
                                        route_group.spatial_data(),
                                        route_group,
                                    );

                                    let user_destination = converter(&Point(coord! {x: to_lon_lat.0, y: to_lon_lat.1}));
                                    Self::destination_pin(api.clone(),
                                                          dpi_scale,
                                                          Some(DVec3::new(user_destination.x(), user_destination.y(), 0.0)));
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

    fn get_style_for_route(alternative: bool, costing: &RouteCosting) -> StyleId {
        match costing {
            RouteCosting::Pedestrian => {
                if alternative {
                    StyleId::new("route_pedestrian_alternative")
                } else {
                    StyleId::new("route_pedestrian")
                }
            },
            RouteCosting::Auto | RouteCosting::Motorbike => {
                if alternative {
                    StyleId::new("route_alternative")
                } else {
                    StyleId::new("route")
                }
            },
        }
    }

    fn destination_pin(api: Arc<RAPI>, dpi_scale: f32, location: Option<DVec3>) {
        if let Some(mut location) = location {
            let group = GeometryRenderGroup(GeometryData::Svg(IconShapeData {
                id: u64::MAX,
                icon_data: IconData {
                    id: "dest_pin",
                    icon_type: IconType::SvgBinary(None, Self::PIN_SVG),
                },
                position: Default::default(),
                size: 60.0 * dpi_scale,
                with_collision: false,
                background: None,
            }));
            // move it a bit so tip of the pin would be close to destination location
            location.y -= 1.0;
            api.add_render_group(
                "dest_pin".to_string(),
                SpatialData::transform(location),
                group,
            );
        } else {
            api.clear_render_groups(HashSet::from_iter(vec!["dest_pin".to_string()]));
        }
    }

    pub fn handle_trips(trips: (Trip, Vec<Trip>), mut action: impl FnMut(u8, &Trip) -> ()) {
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
        // TODO Need to cancel requests too later
        let active_routes = self.get_active_route_ids().clone();
        self.active_routes.store(0, Ordering::Relaxed);
        self.active_routes_ids.clear();
        Self::destination_pin(api.clone(), self.dpi_scale, None);
        api.clear_render_groups(HashSet::from_iter(active_routes));
    }

    pub fn try_change_route_for_world_point(&mut self, point: Point, max_dist: f32) {
        println!("try_change_route_for_world_point for = {:?} with max_dist: {max_dist}", point);

        if let Ok(full_routes) = self.full_routes.try_read() {
            let qq = full_routes.iter().map(|route| {
                let dist = Euclidean.distance(&route.route, &point);
                (route.id.clone(), dist)
            }).min_by(|(_, dist1), (_, dist2)| dist1.total_cmp(dist2));
            if let Some((target_id, dist)) = qq && dist <= max_dist as f64 {
                let api = Arc::clone(&self.api);
                full_routes.iter().for_each(|route| {
                    let route_id = route.id.clone();
                    if target_id == route_id {
                        api.copy_style(StyleId::new("route"), StyleId(route_id.clone().into()));
                        api.bring_to_front(route_id, Some("route_layer".to_string()));
                    } else {
                        api.copy_style(StyleId::new("route_alternative"), StyleId(route_id.into()));
                    }
                });
            }
        }
    }

    pub fn update(&mut self, normal_scale: f64) {
        let api = Arc::clone(&self.api);
        self.get_active_route_ids().iter().cloned().for_each(|id| {
            api.update_spatial_data(id, move |spatial_data| {
                spatial_data.normal_scale = normal_scale;
            });
        });
    }

    fn get_active_route_ids(&mut self) -> &Vec<String> {
        if self.active_routes_ids.is_empty() {
            let active_routes = self.active_routes.load(Ordering::Relaxed);
            if active_routes > 0 {
                self.active_routes_ids.extend((0..active_routes).map(Self::create_route_id));
            }
        }
        &self.active_routes_ids
    }

    fn create_route_id(index: u8) -> String {
        format!("route{index}").to_string()
    }
}
