use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::view_projection::ViewProjection;
use renderer_common::collision_handler::CollisionHandler;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::spawn;

enum ColliderMsg {
    ViewProj(ViewProjection),
}

pub(crate) trait ColliderTask: Send {
    fn run(&mut self, view_projection: &ViewProjection, collision_handler: &mut CollisionHandler);
}

pub(crate) struct Collider {
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
                // drop all ViewProj but the last one
                let mut last_msg = None;
                while let Ok(msg) = receiver.try_recv() {
                    last_msg = Some(msg);
                }

                if let Some(msg) = last_msg {
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

    pub(crate) fn update_view_proj(&mut self, view_projection: &ViewProjection) {
        self.sender
            .send(ColliderMsg::ViewProj(view_projection.clone()))
            .unwrap();
    }
}

pub struct CollisionTaskController<T, R> {
    pub sender: Sender<Box<dyn FnOnce(&mut RenderDataHolder<T>) + Send + 'static>>,
    pub receiver: Receiver<R>,
}

impl<T: Send + 'static, R> CollisionTaskController<T, R> {
    pub(crate) fn update_data<F: FnOnce(&mut RenderDataHolder<T>) + Send + 'static>(
        &self,
        updater: F,
    ) {
        self.sender.send(Box::new(updater)).unwrap();
    }
    pub(crate) fn clear_by_key(&mut self, key: &str) {
        let key = key.to_string();
        self.sender
            .send(Box::new(move |holder| holder.remove(key.as_str())))
            .unwrap();
    }
}

pub struct CollisionTaskWrapper<T, R> {
    render_data_holder: RenderDataHolder<T>,
    data_rx: Arc<Mutex<Receiver<Box<dyn FnOnce(&mut RenderDataHolder<T>) + Send + 'static>>>>,
    result_tx: Sender<R>,
}

impl<T, R> CollisionTaskWrapper<T, R> {
    pub fn new() -> (Self, CollisionTaskController<T, R>) {
        let (data_tx, data_rx) = channel();
        let (result_tx, result_rx) = channel();

        (
            Self {
                render_data_holder: RenderDataHolder::new(),
                data_rx: Arc::new(Mutex::new(data_rx)),
                result_tx,
            },
            CollisionTaskController {
                sender: data_tx,
                receiver: result_rx,
            },
        )
    }

    pub fn update_holder(&mut self) -> &mut RenderDataHolder<T> {
        while let Ok(data) = self.data_rx.lock().unwrap().try_recv() {
            data(&mut self.render_data_holder);
        }
        &mut self.render_data_holder
    }

    pub fn send_result(&self, result: R) {
        self.result_tx.send(result).unwrap();
    }
}
