use std::time::Instant;

/// First in first out fixed size (N) queue
pub struct Fifo<const N: usize, T> {
    pub items: [T; N],
    pub index: usize,
}

impl<const N: usize, T: Copy + Default> Fifo<N, T> {
    pub fn new() -> Self {
        Self {
            items: [T::default(); N],
            index: 0,
        }
    }

    /// Push item to the queue. Returns prev item(replaced by new one)
    pub fn push(&mut self, item: T) -> T {
        let prev = self.items[self.index];
        self.items[self.index] = item;
        self.index = (self.index + 1) % N;
        prev
    }
}

/// FPS counter, N is how many frame samples to use (less = more accurate, more = more stable)
pub struct FpsCounter<const N: usize> {
    last: Instant,
    samples: Fifo<N, f64>,
    running_sum: f64,
}

impl<const N: usize> FpsCounter<N> {
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
            samples: Fifo::new(),
            running_sum: 0f64,
        }
    }

    pub fn update(&mut self) -> f64 {
        let now = Instant::now();
        let new_sample = (now - self.last).as_secs_f64();
        let prev_sample = self.samples.push(new_sample);
        self.running_sum -= prev_sample;
        self.running_sum += new_sample;
        self.last = now;

        1.0 / (self.running_sum / N as f64)
    }
}
