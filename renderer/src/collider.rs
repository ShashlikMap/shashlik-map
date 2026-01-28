use crate::collision_handler::CollisionHandler;
use crate::view_projection::ViewProjection;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::spawn;

enum ColliderMsg {
    ViewProj(ViewProjection),
}

pub trait ColliderTask: Send {
    fn run(&mut self, view_projection: &ViewProjection, collision_handler: &mut CollisionHandler);
}

pub struct Collider {
    tasks: Arc<Mutex<Vec<Box<dyn ColliderTask>>>>,
    sender: Sender<ColliderMsg>,
}

impl Collider {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let tasks = Arc::new(Mutex::new(vec![]));
        Self::run_background(Arc::clone(&tasks), receiver);
        Self { tasks, sender }
    }

    pub fn register_task(&mut self, task: Box<dyn ColliderTask>) {
        self.tasks.lock().unwrap().push(task);
    }

    fn run_background(
        tasks: Arc<Mutex<Vec<Box<dyn ColliderTask>>>>,
        receiver: Receiver<ColliderMsg>,
    ) {
        spawn(move || {
            loop {
                if let Some(msg) = receiver.recv().ok() {
                    match msg {
                        ColliderMsg::ViewProj(view_projection) => {
                            let (width, height) = view_projection.screen_size;
                            let mut collision_handler =
                                CollisionHandler::new(width as f32, height as f32);
                            if let Ok(mut tasks) = tasks.try_lock() {
                                tasks.iter_mut().for_each(|task| {
                                    task.run(&view_projection, &mut collision_handler);
                                })
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn update_view_proj(&mut self, view_projection: &ViewProjection) {
        self.sender
            .send(ColliderMsg::ViewProj(view_projection.clone()))
            .unwrap();
    }
}
