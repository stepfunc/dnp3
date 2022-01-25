enum Value<T> {
    Ready(T),
    Async(core::pin::Pin<Box<dyn core::future::Future<Output = T> + Send + 'static>>),
}

/// Represents a result that may be computed synchronously or asynchronously
///
/// Rust does not currently allow `async fn` in trait methods, we need
/// a workaround. There are crates such as `async_trait` which provide proc_macros
/// that do this, but they don't provide an optimization to *avoid* the heap allocation
/// if the underlying implementation is synchronous.
///
/// This allows us to use `async` operations in Rust if desired, but just have synchronous callbacks
/// in the FFI without a heap allocation
#[must_use]
pub struct MaybeAsync<T> {
    inner: Value<T>,
}

impl<T> MaybeAsync<T> {
    /// retrieve the value, which might be available immediately or require awaiting
    pub async fn get(self) -> T {
        match self.inner {
            Value::Ready(x) => x,
            Value::Async(x) => x.await,
        }
    }

    /// construct a new `MaybeAsync` from an already available result
    pub fn ready(result: T) -> Self {
        MaybeAsync {
            inner: Value::Ready(result),
        }
    }

    /// construct a new `MaybeAsync` from a future which yields the value eventually
    pub fn asynchronous<F>(result: F) -> Self
    where
        F: core::future::Future<Output = T> + Send + 'static,
    {
        MaybeAsync {
            inner: Value::Async(Box::pin(result)),
        }
    }
}
