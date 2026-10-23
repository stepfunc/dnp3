// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

/// Controls the decoding of transmitted and received data at the application, transport, and link layer
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
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
#[derive(Default)]
pub enum AppDecodeLevel {
    /// Decode nothing
    #[default]
    Nothing,
    /// Decode the header-only
    Header,
    /// Decode the header and the object headers
    ObjectHeaders,
    /// Decode the header, the object headers, and the object values
    ObjectValues,
}

/// Controls how transmitted and received transport segments are decoded at the INFO log level
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Default)]
pub enum TransportDecodeLevel {
    /// Decode nothing
    #[default]
    Nothing,
    /// Decode the header
    Header,
    /// Decode the header and the raw payload as hexadecimal
    Payload,
}

/// Controls how transmitted and received link frames are decoded at the INFO log level
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Default)]
pub enum LinkDecodeLevel {
    /// Decode nothing
    #[default]
    Nothing,
    /// Decode the header
    Header,
    /// Decode the header and the raw payload as hexadecimal
    Payload,
}

/// Controls how data transmitted at the physical layer (TCP, serial, etc) is logged
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Default)]
pub enum PhysDecodeLevel {
    /// Log nothing
    #[default]
    Nothing,
    /// Log only the length of data that is sent and received
    Length,
    /// Log the length and the actual data that is sent and received
    Data,
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

impl From<AppDecodeLevel> for DecodeLevel {
    fn from(application: AppDecodeLevel) -> Self {
        Self {
            application,
            ..Self::default()
        }
    }
}

impl AppDecodeLevel {
    pub(crate) fn enabled(&self) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    const NOTHING: DecodeLevel = DecodeLevel {
        application: AppDecodeLevel::Nothing,
        transport: TransportDecodeLevel::Nothing,
        link: LinkDecodeLevel::Nothing,
        physical: PhysDecodeLevel::Nothing,
    };

    #[test]
    fn default_and_an_application_level_leave_the_other_layers_off() {
        assert_eq!(DecodeLevel::default(), NOTHING);
        assert_eq!(DecodeLevel::nothing(), NOTHING);
        assert_eq!(
            DecodeLevel::from(AppDecodeLevel::ObjectValues),
            DecodeLevel {
                application: AppDecodeLevel::ObjectValues,
                ..NOTHING
            }
        );
    }

    #[test]
    fn every_level_above_nothing_is_enabled() {
        let app = [
            AppDecodeLevel::Nothing,
            AppDecodeLevel::Header,
            AppDecodeLevel::ObjectHeaders,
            AppDecodeLevel::ObjectValues,
        ];
        assert_eq!(app.map(|x| x.enabled()), [false, true, true, true]);
        let transport = [
            TransportDecodeLevel::Nothing,
            TransportDecodeLevel::Header,
            TransportDecodeLevel::Payload,
        ];
        assert_eq!(transport.map(|x| x.enabled()), [false, true, true]);
        let link = [
            LinkDecodeLevel::Nothing,
            LinkDecodeLevel::Header,
            LinkDecodeLevel::Payload,
        ];
        assert_eq!(link.map(|x| x.enabled()), [false, true, true]);
        let physical = [
            PhysDecodeLevel::Nothing,
            PhysDecodeLevel::Length,
            PhysDecodeLevel::Data,
        ];
        assert_eq!(physical.map(|x| x.enabled()), [false, true, true]);
    }
}
