use std::sync::atomic::Ordering;

static CLIENT_TCP_NO_DELAY: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);
static SERVER_TCP_NO_DELAY: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

pub(crate) fn configure_client(stream: &tokio::net::TcpStream) {
    configure_no_delay(CLIENT_TCP_NO_DELAY.load(Ordering::Relaxed), stream)
}

pub(crate) fn configure_server(stream: &tokio::net::TcpStream) {
    configure_no_delay(SERVER_TCP_NO_DELAY.load(Ordering::Relaxed), stream)
}

fn configure_no_delay(value: bool, stream: &tokio::net::TcpStream) {
    if let Err(err) = stream.set_nodelay(value) {
        tracing::info!("Unable to set TCP_NODELAY to {}: {}", value, err);
    }
}

/// By default, TCP_NODELAY is set to true for all client TCP/TLS connections. This disables Nagle's
/// algorithm causing the OS to send data written to socket ASAP without waiting. This reduces
/// latency and is usually the appropriate setting for DNP3. This library always writes data
/// in units of link-layer frames so the default setting might cause more TCP fragmentation
/// if clients send requests that exceed a single link-layer frame.
///
/// Calling this function will enable Nagle's algorithm for all future outbound TCP connections.
/// This would typically be called prior to creating any TCP/TLS clients.
///
/// In a future 2.0 release, this flag will likely be settable on a per-session basis but is done
/// globally to preserve API compatibility
pub fn disable_client_tcp_no_delay() {
    CLIENT_TCP_NO_DELAY.store(false, Ordering::Relaxed)
}

/// By default, TCP_NODELAY is set to true for all server TCP/TLS connections. This disables Nagle's
/// algorithm causing the OS to send data written to socket ASAP without waiting. This reduces
/// latency and is usually the appropriate setting for DNP3. This library always writes data
/// in units of link-layer frames so the default setting might cause more TCP fragmentation
/// if clients send requests that exceed a single link-layer frame.
///
/// Calling this function will enable Nagle's algorithm for all future TCP connections accepted by servers.
/// This would typically be called prior to creating any TCP/TLS servers.
///
/// In a future 2.0 release, this flag will likely be settable on a per-session basis but is done
/// globally to preserve API compatibility
pub fn disable_server_tcp_no_delay() {
    SERVER_TCP_NO_DELAY.store(false, Ordering::Relaxed)
}
