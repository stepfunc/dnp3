use oo_bindgen::model::{doc, BackTraced, LibraryBuilder};

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<()> {
    let disable_client_tcp_no_delay = lib
        .define_function("disable_client_tcp_no_delay")?
        .doc(
            doc("By default, TCP_NODELAY is set to true for all client TCP/TLS connections. This disables Nagle's algorithm causing the OS to send data written to socket ASAP without waiting. This reduces latency and is usually the appropriate setting for DNP3. This library always writes data in units of link-layer frames so the default setting might cause more TCP fragmentation if clients send requests that exceed a single link-layer frame")
                .details("Calling this function will enable Nagle's algorithm for all future outbound TCP connections.")
                .details("This would typically be called prior to creating any TCP/TLS clients. In a future 2.0 release, this flag will likely be settable on a per-session basis but is done globally to preserve API compatibility")
        )?
        .build_static("disable_client_tcp_no_delay")?;

    let disable_server_tcp_no_delay = lib
        .define_function("disable_server_tcp_no_delay")?
        .doc(
            doc("By default, TCP_NODELAY is set to true for all server TCP/TLS connections. This disables Nagle's algorithm causing the OS to send data written to socket ASAP without waiting. This reduces latency and is usually the appropriate setting for DNP3. This library always writes data in units of link-layer frames so the default setting might cause more TCP fragmentation if clients send requests that exceed a single link-layer frame")
                .details("Calling this function will enable Nagle's algorithm for all future TCP/TLS connections accepted by servers")
                .details("This would typically be called prior to creating any TCP/TLS clients. In a future 2.0 release, this flag will likely be settable on a per-session basis but is done globally to preserve API compatibility")
        )?
        .build_static("disable_server_tcp_no_delay")?;

    lib.define_static_class("tcp_settings")?
        .static_method(disable_client_tcp_no_delay)?
        .static_method(disable_server_tcp_no_delay)?
        .doc("Global TCP/TLS configuration settings")?
        .build()?;

    Ok(())
}
