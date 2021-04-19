/// A generic listener type that can be invoked multiple times.
/// The user can select to implement it using FnMut, Watch, or not at all.
pub enum Listener<T> {
    /// nothing is listening
    None,
    /// listener is a boxed FnMut
    BoxedFn(Box<dyn FnMut(T) + Send + Sync>),
    /// listener is a broadcast channel
    Watch(crate::tokio::sync::broadcast::Sender<T>),
}

impl<T> Listener<T> {
    pub(crate) fn update(&mut self, value: T) {
        match self {
            Listener::None => {}
            Listener::BoxedFn(func) => func(value),
            Listener::Watch(s) => {
                s.send(value).ok();
            }
        }
    }
}
