use std::time::Instant;

/// First in first out fixed size (N) queue
pub struct Fifo<const N: usize, T> {
    pub items: [T; N],
    pub size: usize,
}

impl<const N: usize, T: Copy + Default> Fifo<N, T> {
    pub fn new() -> Self {
        Self {
            items: [T::default(); N],
            size: 0,
        }
    }

    /// Push item to the queue
    pub fn push(&mut self, item: T) {
        if self.size != N {
            self.items[self.size] = item;
            self.size += 1;
        } else {
            for i in (1..N).rev() {
                self.items[i] = self.items[i - 1];
            }
            self.items[0] = item;
        }
    }
}

/// FPS counter, N is how many frame samples to use (less = more accurate, more = more stable)
pub struct FpsCounter<const N: usize> {
    last: Instant,
    samples: Fifo<N, f64>,
}

impl<const N: usize> FpsCounter<N> {
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
            samples: Fifo::new(),
        }
    }

    pub fn update(&mut self) -> f64 {
        let now = Instant::now();
        let elapsed = now - self.last;
        self.samples.push(elapsed.as_secs_f64());
        self.last = now;

        1.0 / (self.samples.items.iter().copied().sum::<f64>() / N as f64)
    }
}