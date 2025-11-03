use geo_types::Point;
use rstar::primitives::Rectangle;
use rstar::{Envelope, RTree, RTreeObject};

pub struct CollisionHandler {
    objects: RTree<Rectangle<Point<f32>>>,
}

impl CollisionHandler {
    pub fn new() -> Self {
        CollisionHandler {
            objects: RTree::new(),
        }
    }

    pub fn insert(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        rectangle: Rectangle<Point<f32>>,
    ) -> Option<bool> {
        let screen_rect: Rectangle<Point<f32>> = Rectangle::from_corners(
            (0.0, 0.0).into(),
            (config.width as f32, config.height as f32).into(),
        );
        let envelope = rectangle.envelope();
        if screen_rect.envelope().contains_envelope(&envelope) {
            let count = self
                .objects
                .locate_in_envelope_intersecting(&envelope)
                .count();
            self.objects.insert(rectangle);
            Some(count <= 0)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.objects = RTree::new();
    }
}
