/// Controls how errors in parsed link-layer frames are handled. This behavior
/// is configurable for physical layers with built-in error correction like TCP
/// as the connection might be through a terminal server.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum LinkErrorMode {
    /// Framing errors are discarded. The link-layer parser is reset on any error, and the
    /// parser begins scanning for 0x0564. This is always the behavior for serial ports.
    Discard,
    /// Framing errors are bubbled up to calling code, closing the session. Suitable for physical
    /// layers that provide error correction like TCP.
    Close,
}
