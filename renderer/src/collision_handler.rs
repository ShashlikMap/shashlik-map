use geo_types::Point;
use rstar::primitives::Rectangle;
use rstar::{Envelope, RTree, RTreeObject};

pub struct CollisionHandler {
    objects: RTree<Rectangle<Point<f32>>>,
    screen_rect: Rectangle<Point<f32>>,
}

impl CollisionHandler {
    pub fn new(width: f32, height: f32) -> Self {
        CollisionHandler {
            objects: RTree::new(),
            screen_rect: Rectangle::from_corners(Point::new(0.0, 0.0), Point::new(width, height)),
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
        if !self.check_rectangle(&rectangle) {
            return false
        }

        self.objects.insert(rectangle);
        true
    }

    pub fn insert_rectangles(&mut self, rectangles: Vec<Rectangle<Point<f32>>>) -> bool {
        for rect in &rectangles {
            if !self.check_rectangle(rect) {
                return false
            }
        }

        rectangles.into_iter().for_each(|rect| {
            self.objects.insert(rect);
        });
        true
    }

    fn check_rectangle(&self, rectangle: &Rectangle<Point<f32>>) -> bool {
        let envelope = rectangle.envelope();
        // no need to count all items, non-empty check is enough
        let has_items = self
            .objects
            .locate_in_envelope_intersecting(&envelope)
            .peekable().peek().is_some();
        if has_items {
            return false;
        }
        true
    }

    pub fn clear(&mut self) {
        self.objects = RTree::new();
    }
}
