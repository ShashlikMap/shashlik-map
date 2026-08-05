use crossbeam_channel::{Receiver, bounded};
use crossbeam_queue::SegQueue;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

type ModificationClosure<B> = Box<dyn FnOnce(&mut B) + Send + 'static>;

pub struct WorkerHandler<B: Send + 'static, P: Send + 'static> {
    instruction_queue: Arc<SegQueue<ModificationClosure<B>>>,
    rx_from_worker: Receiver<P>,
    worker_thread: JoinHandle<()>,
}

impl<B: Default + Send + 'static, P: Send + 'static> WorkerHandler<B, P> {
    pub fn spawn<F>(mut post_process_fn: F) -> Self
    where
        F: FnMut(&mut B) -> P + Send + 'static,
    {
        let instruction_queue = Arc::new(SegQueue::<ModificationClosure<B>>::new());
        let worker_input = Arc::clone(&instruction_queue);
        let (tx_to_renderer, rx_from_worker) = bounded(1);
        let rx_from_worker_cloned = rx_from_worker.clone();
        let worker_thread = thread::spawn(move || {
            let mut local_state = B::default();

            loop {
                let first_closure = match worker_input.pop() {
                    Some(closure) => closure,
                    None => {
                        thread::park();
                        continue;
                    }
                };

                first_closure(&mut local_state);

                while let Some(modify_closure) = worker_input.pop() {
                    modify_closure(&mut local_state);
                }

                let result = post_process_fn(&mut local_state);
                let mut current_result = result;
                loop {
                    match tx_to_renderer.try_send(current_result) {
                        Ok(_) => break,
                        Err(crossbeam_channel::TrySendError::Full(returned_result)) => {
                            let _ = rx_from_worker_cloned.try_recv();
                            current_result = returned_result;
                        }
                        Err(crossbeam_channel::TrySendError::Disconnected(_)) => break,
                    }
                }
            }
        });

        Self {
            instruction_queue,
            rx_from_worker,
            worker_thread,
        }
    }

    pub fn update_data<F>(&self, modify_fn: F)
    where
        F: FnOnce(&mut B) + Send + 'static,
    {
        self.instruction_queue.push(Box::new(modify_fn));
        self.trigger();
    }

    fn trigger(&self) {
        self.worker_thread.thread().unpark();
    }

    pub fn try_get_result(&self) -> Option<P> {
        self.rx_from_worker.try_recv().ok()
    }
}
