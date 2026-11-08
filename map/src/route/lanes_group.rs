use glam::DVec3;
use lyon::geom::euclid::{Box2D, point2};
use lyon::path::builder::BorderRadii;
use lyon::path::{Path, Winding};
use valhalla_client::route::TurnLaneDirection;
use renderer_common::CanvasApi;
use renderer_common::geometry_data::{GeometryData, IconBackground, IconData, IconShapeData, IconType, Svg};
use renderer_common::render_group::RenderGroup;
use renderer_common::style_id::StyleId;

pub(crate) struct LanesInfo {
    pub pos: DVec3,
    pub dirs: Vec<Vec<TurnLaneDirection>>
}

pub(crate) struct LanesGroup {
    pub(crate) pos: Vec<LanesInfo>,
}
const TURN_LEFT_SVG: Svg = Svg("turn_left_svg", include_bytes!("../../svg/turn_left.svg"));
const TURN_MERGE_TO_LEFT_SVG: Svg = Svg("turn_merge_to_left_svg", include_bytes!("../../svg/turn_merge_to_left.svg"));
const TURN_MERGE_TO_RIGHT_SVG: Svg = Svg("turn_merge_to_right_svg", include_bytes!("../../svg/turn_merge_to_right.svg"));
const TURN_REVERSE_SVG: Svg = Svg("turn_reverse_svg", include_bytes!("../../svg/turn_reverse.svg"));
const TURN_RIGHT_SVG: Svg = Svg("turn_right_svg", include_bytes!("../../svg/turn_right.svg"));
const TURN_SHARP_LEFT_SVG: Svg = Svg("turn_sharp_left_svg", include_bytes!("../../svg/turn_sharp_left.svg"));
const TURN_SHARP_RIGHT_SVG: Svg = Svg("turn_sharp_right_svg", include_bytes!("../../svg/turn_sharp_right.svg"));
const TURN_SLIGHT_LEFT_SVG: Svg = Svg("turn_slight_left_svg", include_bytes!("../../svg/turn_slight_left.svg"));
const TURN_SLIGHT_RIGHT_SVG: Svg = Svg("turn_slight_right_svg", include_bytes!("../../svg/turn_slight_right.svg"));
const TURN_THROUGH_SVG: Svg = Svg("turn_through_svg", include_bytes!("../../svg/turn_through.svg"));

impl LanesGroup {
    fn convert_svg(dir: &TurnLaneDirection) -> Option<Svg> {
        match dir {
            TurnLaneDirection::Empty => None,
            TurnLaneDirection::None => None,
            TurnLaneDirection::Through => Some(TURN_THROUGH_SVG),
            TurnLaneDirection::SharpLeft => Some(TURN_SHARP_LEFT_SVG),
            TurnLaneDirection::Left => Some(TURN_LEFT_SVG),
            TurnLaneDirection::SlightLeft => Some(TURN_SLIGHT_LEFT_SVG),
            TurnLaneDirection::SlightRight => Some(TURN_SLIGHT_RIGHT_SVG),
            TurnLaneDirection::Right => Some(TURN_RIGHT_SVG),
            TurnLaneDirection::SharpRight => Some(TURN_SHARP_RIGHT_SVG),
            TurnLaneDirection::Reverse => Some(TURN_REVERSE_SVG),
            TurnLaneDirection::MergeToLeft => Some(TURN_MERGE_TO_LEFT_SVG),
            TurnLaneDirection::MergeToRight => Some(TURN_MERGE_TO_RIGHT_SVG),
        }
    }
}

impl<T: CanvasApi> RenderGroup<T> for LanesGroup {

    fn content(self: Box<Self>, canvas: &mut T) {
        let padding = 7.0; // * dpi_scale;
        self.pos.iter().enumerate().for_each(|(index, lanes_info)| {
            let icon_data: Vec<_> = lanes_info.dirs.iter().map(|lane_dirs| {
                lane_dirs.iter().flat_map(|dir| {
                    Self::convert_svg(dir)
                }).map(|svg| {
                    IconData {
                        id: svg.0,
                        icon_type: IconType::SvgBinary(Some(StyleId::new("poi_toilet")), svg),
                    }
                }).collect()
            }).collect();

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
                id: u64::MAX - 5 - (index as u64),
                icon_data,
                position: lanes_info.pos,
                size: 50.0,
                with_collision: false,
                background: Some(background),
                permanent: false
            }));
        });
    }
}
