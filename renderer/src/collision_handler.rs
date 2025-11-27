use geo_types::Point;
use rstar::primitives::Rectangle;
use rstar::{AABB, Envelope, RTree, RTreeObject};

pub struct CollisionHandler {
    objects: RTree<Rectangle<Point<f32>>>,
    screen_rect: Rectangle<Point<f32>>,
}

impl CollisionHandler {
    pub fn new() -> Self {
        CollisionHandler {
            objects: RTree::new(),
            screen_rect: Rectangle::from_corners(Point::new(0.0, 0.0), Point::new(0.0, 0.0)),
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.screen_rect = Rectangle::from_corners(Point::new(0.0, 0.0), Point::new(width, height));
    }

    pub fn within_screen(
        &self,
        rectangle: Rectangle<Point<f32>>,
    ) -> bool {
        let envelope = rectangle.envelope();
        self.screen_rect.envelope().intersects(&envelope)
    }

    pub fn insert(&mut self, rectangle: Rectangle<Point<f32>>) -> bool {
        let envelope = rectangle.envelope();
        let count = self
            .objects
            .locate_in_envelope_intersecting(&envelope)
            .count();
        if count > 0 {
            return false;
        }

        self.objects.insert(rectangle);
        true
    }

    pub fn insert_rectangles(&mut self, rectangles: Vec<Rectangle<Point<f32>>>) -> bool {
        let envelopes: Vec<AABB<Point<f32>>> =
            rectangles.iter().map(|rect| rect.envelope()).collect();

        for envelope in &envelopes {
            let count = self
                .objects
                .locate_in_envelope_intersecting(envelope)
                .count();
            if count > 0 {
                return false;
            }
        }

        rectangles.into_iter().for_each(|rect| {
            self.objects.insert(rect);
        });
        true
    }

    pub fn clear(&mut self) {
        self.objects = RTree::new();
    }
}
