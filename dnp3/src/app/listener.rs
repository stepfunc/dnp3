/// A generic listener type that can be invoked multiple times
pub trait Listener<T>: Send {
    /// inform the listener that the value has changed
    fn update(&mut self, value: T);
}

/// Listener that does nothing
#[derive(Copy, Clone)]
pub struct NullListener;

impl NullListener {
    /// create a Box<dyn Listener<T>> that does nothing
    pub fn create<T>() -> Box<dyn Listener<T>> {
        Box::new(NullListener)
    }
}

impl<T> Listener<T> for NullListener {
    fn update(&mut self, _value: T) {}
}
