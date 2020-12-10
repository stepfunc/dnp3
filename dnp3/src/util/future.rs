use std::task::Context;

struct NeverReady;

impl std::future::Future for NeverReady {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, _: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        std::task::Poll::Pending
    }
}

pub(crate) fn forever() -> impl std::future::Future<Output = ()> {
    NeverReady {}
}
