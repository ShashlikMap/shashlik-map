use crate::collision_handler::CollisionHandler;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::view_projection::ViewProjection;
use cgmath::num_traits::clamp;
use cgmath::Vector3;
use geo_types::point;
use rstar::primitives::Rectangle;
use std::collections::HashMap;
use std::mem;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread::spawn;

enum ColliderMsg {
    Resize(f32, f32),
    ViewProj(ViewProjection),
    Clear(String),
    InstanceData1(Vec<(String, (Vector3<f64>, String))>),
}

pub trait ColliderTask: Send {
    fn run(&mut self, view_projection: &ViewProjection, collision_handler: &mut CollisionHandler);
}

pub struct Collider {
    tasks: Arc<Mutex<Vec<Box<dyn ColliderTask>>>>,
    sender: Sender<ColliderMsg>,
    result1: Arc<RwLock<HashMap<String, Vec<(Vector3<f64>, f32)>>>>,
}

impl Collider {
    pub fn new(width: f32, height: f32) -> Self {
        let collision_handler = CollisionHandler::new(width, height);
        let (sender, receiver) = mpsc::channel();
        let result1 = Arc::new(RwLock::new(HashMap::new()));
        let tasks = Arc::new(Mutex::new(vec![]));
        Self::run_background(Arc::clone(&tasks), Arc::clone(&result1), collision_handler, receiver);
        Self {
            tasks,
            sender,
            result1,
        }
    }

    pub fn register_task(&mut self, task: Box<dyn ColliderTask>) {
        self.tasks.lock().unwrap().push(task);
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.sender
            .send(ColliderMsg::Resize(width, height))
            .unwrap();
    }

    fn run_background(
        tasks: Arc<Mutex<Vec<Box<dyn ColliderTask>>>>,
        result1: Arc<RwLock<HashMap<String, Vec<(Vector3<f64>, f32)>>>>,
        mut collision_handler: CollisionHandler,
        receiver: Receiver<ColliderMsg>,
    ) {
        spawn(move || {
            let mut instance_data1: RenderDataHolder<(Vector3<f64>, f32, String)> =
                RenderDataHolder::new();
            loop {
                if let Some(msg) = receiver.recv().ok() {
                    match msg {
                        ColliderMsg::ViewProj(view_projection) => {
                            if let Ok(mut tasks) = tasks.try_lock() {
                                tasks.iter_mut().for_each(|task| {
                                    task.run(&view_projection, &mut collision_handler);
                                })
                            }

                            let mut hm: HashMap<String, Vec<(Vector3<f64>, f32)>> = HashMap::new();
                            instance_data1.run_mut_action(|(pos, alpha, key)| {
                                let screen_pos = view_projection.screen_position(&pos);
                                // TODO Bounds for svg?
                                // no need to use f64 for collision detection
                                let bounds = Rectangle::from_corners(
                                    point! { x: screen_pos.x as f32 - 20.0, y: screen_pos.y as f32 - 20.0},
                                    point! { x: screen_pos.x as f32 + 20.0, y: screen_pos.y as f32 + 20.0},
                                );

                                let within_screen = collision_handler.within_screen(bounds);
                                if within_screen {
                                    if collision_handler.insert(bounds) {
                                        *alpha = clamp(*alpha + 0.05, 0.0, 1.0);
                                    } else {
                                        *alpha = clamp(*alpha - 0.05, 0.0, 1.0);
                                    }
                                }

                                hm.entry(key.clone()).or_default().push((*pos, *alpha));
                            });
                            *result1.write().unwrap() = hm;
                            collision_handler.clear();
                        }
                        ColliderMsg::InstanceData1(data) => {
                            data.into_iter().for_each(|item| {
                                let key = item.0;
                                let (position, instance_key) = item.1;
                                instance_data1.add(key, (position, 0.0, instance_key));
                            });
                        }

                        ColliderMsg::Resize(width, height) => collision_handler.resize(width, height),
                        ColliderMsg::Clear(key) => instance_data1.remove(key.as_str()),
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

    pub fn set_data1(&mut self, data: Vec<(String, (Vector3<f64>, String))>) {
        self.sender.send(ColliderMsg::InstanceData1(data)).unwrap();
    }

    pub fn get_result1(&self) -> Option<HashMap<String, Vec<(Vector3<f64>, f32)>>> {
        let mut res = self.result1.try_write().ok()?;
        let res: HashMap<String, Vec<(Vector3<f64>, f32)>> = mem::take(&mut res);
        if res.is_empty() {
            return None
        }
        Some(res)
    }

    pub fn clear_by_key(&mut self, key: &str) {
        self.sender
            .send(ColliderMsg::Clear(key.to_string()))
            .unwrap();
    }
}
