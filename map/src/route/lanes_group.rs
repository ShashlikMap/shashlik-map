use std::cmp::PartialEq;
use glam::DVec3;
use lyon::geom::euclid::{Box2D, point2};
use lyon::path::builder::BorderRadii;
use lyon::path::{Path, Winding};
use rand::RngExt;
use renderer_common::{include_svg, CanvasApi};
use renderer_common::geometry_data::{
    GeometryData, IconBackground, IconData, IconShapeData, IconType, Svg,
};
use renderer_common::render_group::RenderGroup;
use renderer_common::style_id::StyleId;
use valhalla_client::route::{TurnLaneDirection};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum TurnLaneDirectionStatus {
    None,
    Valid,
    Active
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct TurnLaneDirectionWithStatus {
    pub dir: TurnLaneDirection,
    pub status: TurnLaneDirectionStatus
}

pub(crate) struct LanesInfo {
    pub pos: DVec3,
    pub dirs: Vec<Vec<TurnLaneDirectionWithStatus>>,
}

pub(crate) struct LanesGroup {
    pub dpi_scale: f32,
    pub lanes_info: Vec<LanesInfo>,
}

impl LanesGroup {
    fn convert_svg(turn: TurnLaneDirectionWithStatus) -> Option<(TurnLaneDirectionWithStatus, Svg)> {

        match turn.dir {
            TurnLaneDirection::Empty => None,
            TurnLaneDirection::None => None,
            TurnLaneDirection::Through => Some((turn, include_svg!("../../svg/turn_through.svg"))),
            TurnLaneDirection::SharpLeft => Some((turn,include_svg!("../../svg/turn_sharp_left.svg"))),
            TurnLaneDirection::Left => Some((turn,include_svg!("../../svg/turn_left.svg"))),
            TurnLaneDirection::SlightLeft => Some((turn,include_svg!("../../svg/turn_slight_left.svg"))),
            TurnLaneDirection::SlightRight => Some((turn,include_svg!("../../svg/turn_slight_right.svg"))),
            TurnLaneDirection::Right => Some((turn,include_svg!("../../svg/turn_right.svg"))),
            TurnLaneDirection::SharpRight => Some((turn,include_svg!("../../svg/turn_sharp_right.svg"))),
            TurnLaneDirection::Reverse => Some((turn,include_svg!("../../svg/turn_reverse.svg"))),
            TurnLaneDirection::MergeToLeft => Some((turn,include_svg!("../../svg/turn_merge_to_left.svg"))),
            TurnLaneDirection::MergeToRight => Some((turn,include_svg!("../../svg/turn_merge_to_right.svg"))),
        }
    }
}

impl<T: CanvasApi> RenderGroup<T> for LanesGroup {
    fn content(self: Box<Self>, canvas: &mut T) {
        let padding = 7.0 * self.dpi_scale;
        let mut rng = rand::rng();
        self.lanes_info.iter().for_each(|lanes_info| {
            let icon_data: Vec<_> = lanes_info
                .dirs
                .iter()
                .map(|lane_dirs| {
                    lane_dirs
                        .iter()
                        .flat_map(|turn| Self::convert_svg(*turn))
                        .map(|(turn, svg)| {
                            // TODO
                            let dir_style = if turn.status != TurnLaneDirectionStatus::None {
                                StyleId::new("poi_lanes")
                            } else {
                                StyleId::new("poi_traffic_light")
                            };
                            IconData {
                                id: svg.0,
                                icon_type: IconType::SvgBinary(Some(dir_style), svg),
                            }
                        })
                        .collect()
                })
                .collect();

            let icons_len = icon_data.len() as f32;
            let background = IconBackground {
                style_id: StyleId::new("poi_icon_background"),
                shape: Box::new(move |data: &IconShapeData| -> Path {
                    let mut builder = Path::builder();
                    let half_size = padding + data.size / 2.0;
                    let rect = Box2D::new(
                        point2(-icons_len * half_size, -half_size),
                        point2(icons_len * half_size, half_size),
                    );
                    builder.add_rounded_rectangle(
                        &rect,
                        &BorderRadii::new(10.0),
                        Winding::Positive,
                    );
                    builder.build()
                }),
            };

            canvas.geometry_data(GeometryData::Svg(IconShapeData {
                id: rng.random(),
                icon_data,
                position: lanes_info.pos,
                size: 50.0 * self.dpi_scale,
                with_collision: false,
                background: Some(background),
                permanent: false,
            }));
        });
    }
}
