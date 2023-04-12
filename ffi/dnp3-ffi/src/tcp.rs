pub(crate) fn disable_client_tcp_no_delay() {
    dnp3::tcp::disable_client_tcp_no_delay();
}

pub(crate) fn disable_server_tcp_no_delay() {
    dnp3::tcp::disable_server_tcp_no_delay();
}

#[cfg(feature = "tls")]
pub(crate) fn enable_client_name_wildcards() {
    #[cfg(feature = "tls")]
    dnp3::tcp::tls::dangerous::enable_client_name_wildcards()
}
