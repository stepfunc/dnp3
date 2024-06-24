use std::marker::PhantomData;
use std::task::Context;

struct NeverReady<T> {
    _phantom: PhantomData<T>,
}

impl<T> std::future::Future for NeverReady<T> {
    type Output = T;

    fn poll(self: std::pin::Pin<&mut Self>, _: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        std::task::Poll::Pending
    }
}

pub(crate) fn forever<T>() -> impl std::future::Future<Output = T> {
    NeverReady::<T> {
        _phantom: Default::default(),
    }
}
