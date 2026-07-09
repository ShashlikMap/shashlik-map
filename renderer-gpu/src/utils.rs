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