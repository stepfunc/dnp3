// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::sync::atomic::Ordering;

use crate::util::phys::Socket;

static CLIENT_TCP_NO_DELAY: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);
static SERVER_TCP_NO_DELAY: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

pub(crate) fn configure_client(stream: &tokio::net::TcpStream) {
    configure_no_delay(CLIENT_TCP_NO_DELAY.load(Ordering::Relaxed), stream)
}

pub(crate) fn configure_server<S: Socket>(stream: &S) {
    configure_no_delay(SERVER_TCP_NO_DELAY.load(Ordering::Relaxed), stream)
}

fn configure_no_delay<S: Socket>(value: bool, stream: &S) {
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
/// in units of link-layer frames, so the default setting might cause more TCP fragmentation
/// if clients send requests that exceed a single link-layer frame.
///
/// Calling this function will enable Nagle's algorithm for all future TCP connections accepted by servers.
/// This would typically be called before creating any TCP/TLS servers.
///
/// In a future 2.0 release, this flag will likely be settable on a per-session basis but is done
/// globally to preserve API compatibility
pub fn disable_server_tcp_no_delay() {
    SERVER_TCP_NO_DELAY.store(false, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_supplied_value_is_set_on_the_socket() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let stream = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        for value in [false, true, false] {
            configure_no_delay(value, &stream);
            assert_eq!(stream.nodelay().unwrap(), value);
        }
    }
}
