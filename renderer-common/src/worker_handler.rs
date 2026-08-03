use std::sync::mpsc::Receiver;
use std::sync::{Arc, mpsc};
use std::thread;
use std::thread::JoinHandle;
use crossbeam_queue::SegQueue;

type ModificationClosure<B> = Box<dyn FnOnce(&mut B) + Send + 'static>;

pub struct WorkerHandler<B: Send + 'static, P: Send + 'static> {
    instruction_queue: Arc<SegQueue<ModificationClosure<B>>>,
    rx_from_worker: Receiver<P>,
    worker_thread: JoinHandle<()>,
}

impl<B: Default + Send + 'static, P: Send + 'static> WorkerHandler<B, P> {
    pub fn spawn<F>(wait_consumer_result: bool, mut post_process_fn: F) -> Self
    where
        F: FnMut(&mut B) -> P + Send + 'static,
    {
        let instruction_queue = Arc::new(SegQueue::<ModificationClosure<B>>::new());
        let (tx_to_renderer, rx_from_worker) = mpsc::sync_channel(1);
        let worker_input = Arc::clone(&instruction_queue);

        let worker_thread = thread::spawn(move || {
            let mut local_state = B::default();

            loop {
                if worker_input.is_empty() {
                    thread::park();
                }

                while let Some(modify_closure) = worker_input.pop() {
                    modify_closure(&mut local_state);
                }

                let result = post_process_fn(&mut local_state);

                if wait_consumer_result {
                    let _ = tx_to_renderer.send(result);
                } else {
                    let _ = tx_to_renderer.try_send(result);
                };
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
