use crate::app::Shutdown;

pub(crate) struct Receiver<T> {
    inner: tokio::sync::mpsc::Receiver<T>,
}

pub(crate) struct Sender<T> {
    inner: tokio::sync::mpsc::Sender<T>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std::fmt::Debug for Sender<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Sender")
            .field("inner", &self.inner)
            .finish()
    }
}

pub(crate) fn request_channel<T>() -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = tokio::sync::mpsc::channel(16); // default size for all request channels
    (Sender::new(tx), Receiver::new(rx))
}

impl<T> Receiver<T> {
    fn new(inner: tokio::sync::mpsc::Receiver<T>) -> Self {
        Receiver { inner }
    }

    pub(crate) async fn receive(&mut self) -> Result<T, Shutdown> {
        match self.inner.recv().await {
            Some(x) => Ok(x),
            None => Err(Shutdown),
        }
    }

    pub(crate) fn close_and_drain(&mut self) {
        self.inner.close();
        while self.inner.try_recv().is_ok() {}
    }
}

impl<T> Sender<T> {
    fn new(inner: tokio::sync::mpsc::Sender<T>) -> Self {
        Sender { inner }
    }

    pub(crate) async fn send(&mut self, value: T) -> Result<(), Shutdown> {
        self.inner.send(value).await.map_err(|_| Shutdown)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Shutdown {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Shutdown
    }
}
