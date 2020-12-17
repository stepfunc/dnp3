/// makes declaring these where clauses requiring these traits easier
pub trait IOStream: crate::tokio::io::AsyncRead + crate::tokio::io::AsyncWrite + Unpin {}

/// blanket impl for any type implementing the required traits
impl<T> IOStream for T where T: crate::tokio::io::AsyncRead + crate::tokio::io::AsyncWrite + Unpin {}
