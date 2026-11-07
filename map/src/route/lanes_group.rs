use glam::DVec3;
use lyon::geom::euclid::{Box2D, point2};
use lyon::path::builder::BorderRadii;
use lyon::path::{Path, Winding};
use renderer_common::CanvasApi;
use renderer_common::geometry_data::{GeometryData, IconBackground, IconData, IconShapeData, IconType, Svg};
use renderer_common::render_group::RenderGroup;
use renderer_common::style_id::StyleId;

pub(crate) struct LanesGroup {
    pub(crate) pos: Vec<DVec3>,
}
const TOILETS_SVG2: Svg = Svg("toilet_svg", include_bytes!("../../svg/toilet.svg"));

impl<T: CanvasApi> RenderGroup<T> for LanesGroup {
    fn content(self: Box<Self>, canvas: &mut T) {
        let padding = 7.0; // * dpi_scale;
        self.pos.iter().enumerate().for_each(|(index, pos)| {
            let background = IconBackground {
                style_id: StyleId::new("poi_icon_background"),
                shape: Box::new(move |data: &IconShapeData| -> Path {
                    let mut builder = Path::builder();
                    let half_size = padding + data.size / 2.0;
                    let rect = Box2D::new(
                        point2(-3.0 * half_size, -half_size),
                        point2(3.0 * half_size, half_size),
                    );
                    builder.add_rounded_rectangle(
                        &rect,
                        &BorderRadii::new(10.0),
                        Winding::Positive,
                    );
                    builder.build()
                }),
            };
            let icon_data = IconData {
                id: "lane_box",
                icon_type: IconType::None,
            };
            canvas.geometry_data(GeometryData::Svg(IconShapeData {
                id: u64::MAX - 155 - (index as u64),
                icon_data: vec![icon_data],
                position: *pos,
                size: 30.0,
                with_collision: false,
                background: Some(background),
            }));

            let icon_data = IconData {
                id: "lane_box_left",
                icon_type: IconType::SvgBinary(Some(StyleId::new("poi_toilet")), TOILETS_SVG2),
            };
            canvas.geometry_data(GeometryData::Svg(IconShapeData {
                id: u64::MAX - 50 - (index as u64),
                icon_data: vec![icon_data],
                position: *pos - DVec3::new(5.0, 0.0, 0.0),
                size: 30.0,
                with_collision: false,
                background: None
            }));

            let icon_data = IconData {
                id: "lane_box_right",
                icon_type: IconType::SvgBinary(Some(StyleId::new("poi_toilet")), TOILETS_SVG2),
            };
            canvas.geometry_data(GeometryData::Svg(IconShapeData {
                id: u64::MAX - 25 - (index as u64),
                icon_data: vec![icon_data],
                position: *pos - DVec3::new(10.0, 0.0, 0.0),
                size: 30.0,
                with_collision: false,
                background: None
            }));
        });
    }
}
