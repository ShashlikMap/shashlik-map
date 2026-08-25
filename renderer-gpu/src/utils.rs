use tokio::sync::broadcast::error::TryRecvError;

pub trait ReceiverExt<T: Clone> {
    fn no_lagged(&mut self) -> Result<T, TryRecvError>;
}

impl<T: Clone> ReceiverExt<T> for tokio::sync::broadcast::Receiver<T> {
    fn no_lagged(&mut self) -> Result<T, TryRecvError> {
        let result = self.try_recv();
        if let Err(err) = &result {
            match err {
                TryRecvError::Lagged(_) => return self.no_lagged(),
                _ => {}
            }
        }
        result
    }
}

pub(crate) trait TextureExt {
    const U32_SIZE: u32 = size_of::<u32>() as u32;
    fn unpadded_bytes_per_row(&self) -> u32;
    fn padded_bytes_per_row(&self) -> u32;
}

impl TextureExt for wgpu::Texture {
    fn unpadded_bytes_per_row(&self) -> u32 {
        Self::U32_SIZE * self.width()
    }

    fn padded_bytes_per_row(&self) -> u32 {
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        (self.unpadded_bytes_per_row() + alignment - 1) & !(alignment - 1)
    }
}