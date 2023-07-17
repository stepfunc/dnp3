/// Controls the decoding of transmitted and received data at the application, transport, and link layer
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct DecodeLevel {
    /// Controls application layer decoding
    #[cfg_attr(feature = "serialization", serde(default))]
    pub application: AppDecodeLevel,
    /// Controls transport layer decoding
    #[cfg_attr(feature = "serialization", serde(default))]
    pub transport: TransportDecodeLevel,
    /// Controls link layer decoding
    #[cfg_attr(feature = "serialization", serde(default))]
    pub link: LinkDecodeLevel,
    /// Controls the logging of physical layer read/write
    #[cfg_attr(feature = "serialization", serde(default))]
    pub physical: PhysDecodeLevel,
}

/// Controls how transmitted and received application-layer fragments are decoded at the INFO log level
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum AppDecodeLevel {
    /// Decode nothing
    Nothing,
    /// Decode the header-only
    Header,
    /// Decode the header and the object headers
    ObjectHeaders,
    /// Decode the header, the object headers, and the object values
    ObjectValues,
}

impl Default for AppDecodeLevel {
    fn default() -> Self {
        Self::Nothing
    }
}

/// Controls how transmitted and received transport segments are decoded at the INFO log level
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum TransportDecodeLevel {
    /// Decode nothing
    Nothing,
    /// Decode the header
    Header,
    /// Decode the header and the raw payload as hexadecimal
    Payload,
}

impl Default for TransportDecodeLevel {
    fn default() -> Self {
        Self::Nothing
    }
}

/// Controls how transmitted and received link frames are decoded at the INFO log level
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum LinkDecodeLevel {
    /// Decode nothing
    Nothing,
    /// Decode the header
    Header,
    /// Decode the header and the raw payload as hexadecimal
    Payload,
}

impl Default for LinkDecodeLevel {
    fn default() -> Self {
        Self::Nothing
    }
}

/// Controls how data transmitted at the physical layer (TCP, serial, etc) is logged
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum PhysDecodeLevel {
    /// Log nothing
    Nothing,
    /// Log only the length of data that is sent and received
    Length,
    /// Log the length and the actual data that is sent and received
    Data,
}

impl Default for PhysDecodeLevel {
    fn default() -> Self {
        Self::Nothing
    }
}

impl DecodeLevel {
    /// construct a `DecodeLevel` with nothing enabled
    pub fn nothing() -> Self {
        Self::default()
    }

    /// construct a `DecodeLevel` from its fields
    pub fn new(
        application: AppDecodeLevel,
        transport: TransportDecodeLevel,
        link: LinkDecodeLevel,
        physical: PhysDecodeLevel,
    ) -> Self {
        DecodeLevel {
            application,
            transport,
            link,
            physical,
        }
    }
}

impl Default for DecodeLevel {
    fn default() -> Self {
        Self {
            application: AppDecodeLevel::Nothing,
            transport: TransportDecodeLevel::Nothing,
            link: LinkDecodeLevel::Nothing,
            physical: PhysDecodeLevel::Nothing,
        }
    }
}

impl From<AppDecodeLevel> for DecodeLevel {
    fn from(application: AppDecodeLevel) -> Self {
        Self {
            application,
            transport: TransportDecodeLevel::Nothing,
            link: LinkDecodeLevel::Nothing,
            physical: PhysDecodeLevel::Nothing,
        }
    }
}

impl AppDecodeLevel {
    pub(crate) fn enabled(&self) -> bool {
        self.header()
    }

    pub(crate) fn header(&self) -> bool {
        match self {
            AppDecodeLevel::Nothing => false,
            AppDecodeLevel::Header => true,
            AppDecodeLevel::ObjectHeaders => true,
            AppDecodeLevel::ObjectValues => true,
        }
    }

    pub(crate) fn object_headers(&self) -> bool {
        match self {
            AppDecodeLevel::Nothing => false,
            AppDecodeLevel::Header => false,
            AppDecodeLevel::ObjectHeaders => true,
            AppDecodeLevel::ObjectValues => true,
        }
    }

    pub(crate) fn object_values(&self) -> bool {
        match self {
            AppDecodeLevel::Nothing => false,
            AppDecodeLevel::Header => false,
            AppDecodeLevel::ObjectHeaders => false,
            AppDecodeLevel::ObjectValues => true,
        }
    }
}

impl TransportDecodeLevel {
    pub(crate) fn enabled(&self) -> bool {
        self.header_enabled()
    }

    pub(crate) fn header_enabled(&self) -> bool {
        match self {
            TransportDecodeLevel::Nothing => false,
            TransportDecodeLevel::Header => true,
            TransportDecodeLevel::Payload => true,
        }
    }

    pub(crate) fn payload_enabled(&self) -> bool {
        match self {
            TransportDecodeLevel::Nothing => false,
            TransportDecodeLevel::Header => false,
            TransportDecodeLevel::Payload => true,
        }
    }
}

impl LinkDecodeLevel {
    pub(crate) fn enabled(&self) -> bool {
        self.header_enabled()
    }

    pub(crate) fn header_enabled(&self) -> bool {
        match self {
            LinkDecodeLevel::Nothing => false,
            LinkDecodeLevel::Header => true,
            LinkDecodeLevel::Payload => true,
        }
    }

    pub(crate) fn payload_enabled(&self) -> bool {
        match self {
            LinkDecodeLevel::Nothing => false,
            LinkDecodeLevel::Header => false,
            LinkDecodeLevel::Payload => true,
        }
    }
}

impl PhysDecodeLevel {
    pub(crate) fn enabled(&self) -> bool {
        self.length_enabled()
    }

    pub(crate) fn length_enabled(&self) -> bool {
        match self {
            PhysDecodeLevel::Nothing => false,
            PhysDecodeLevel::Length => true,
            PhysDecodeLevel::Data => true,
        }
    }

    pub(crate) fn data_enabled(&self) -> bool {
        match self {
            PhysDecodeLevel::Nothing => false,
            PhysDecodeLevel::Length => false,
            PhysDecodeLevel::Data => true,
        }
    }
}
