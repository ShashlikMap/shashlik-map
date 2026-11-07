pub(crate) mod route_controller;
mod lanes_group;

#[derive(Clone)]
pub enum RouteCosting {
    Auto, Pedestrian, Motorbike, 
}
