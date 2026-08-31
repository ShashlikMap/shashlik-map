use geo_types::Point;
use glam::Vec2;
use rstar::primitives::Rectangle;
use rstar::{Envelope, RTree, RTreeObject, AABB};

pub struct CollisionHandler {
    objects: RTree<Rectangle<Point<f32>>>,
    screen_rect_envelope: AABB<Point<f32>>,
    screen_radius_sq: Option<f32>
}

impl CollisionHandler {
    pub fn new(width: f32, height: f32, screen_radius_sq: Option<f32>) -> Self {
        CollisionHandler {
            objects: RTree::new(),
            screen_rect_envelope: Self::create_rect(width, height).envelope(),
            screen_radius_sq
        }
    }

    fn create_rect(width: f32, height: f32) -> Rectangle<Point<f32>> {
        Rectangle::from_corners(Point::new(0.0, 0.0), Point::new(width, height))
    }

    pub fn within_screen(
        &self,
        rectangle: Rectangle<Point<f32>>,
    ) -> bool {
        let envelope = rectangle.envelope();
        if let Some(screen_radius_sq) = self.screen_radius_sq {
            return self.check_screen_radius(screen_radius_sq, envelope);
        }

        self.screen_rect_envelope.intersects(&envelope)
    }

    pub fn point_within_screen(
        &self,
        point: &Vec2,
    ) -> bool {
        let envelope = Point::new(point.x, point.y).envelope();
        if let Some(screen_radius_sq) = self.screen_radius_sq {
            return self.check_screen_radius(screen_radius_sq, envelope);
        }
        self.screen_rect_envelope.intersects(&envelope)
    }

    fn check_screen_radius(&self, screen_radius_sq: f32, other_point_aabb: AABB<Point<f32>>) -> bool {
        let dist_sq_to_screen_center = other_point_aabb.distance_2(&self.screen_rect_envelope.center());
        dist_sq_to_screen_center <= screen_radius_sq
    }

    /// Method first check if rectangle intersects anything and only then adds it to R-tree
    pub fn check_and_insert(&mut self, rectangle: Rectangle<Point<f32>>) -> bool {
        if !self.check_rectangle(&rectangle) {
            return false
        }

        self.objects.insert(rectangle);
        true
    }

    /// Method first check if any of rectangles intersects previously added data to r-tree and only then adds rectangles to R-tree
    pub fn check_and_insert_rectangles(&mut self, rectangles: Vec<Rectangle<Point<f32>>>) -> bool {
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
            .next().is_some();
        !has_items
    }

    pub fn clear(&mut self) {
        self.objects = RTree::new();
    }
}
