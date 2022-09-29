use dnp3::decode::*;

impl From<crate::ffi::DecodeLevel> for DecodeLevel {
    fn from(from: crate::ffi::DecodeLevel) -> Self {
        Self {
            application: from.application().into(),
            transport: from.transport().into(),
            link: from.link().into(),
            physical: from.physical().into(),
        }
    }
}

impl From<DecodeLevel> for crate::ffi::DecodeLevel {
    fn from(from: DecodeLevel) -> Self {
        crate::ffi::DecodeLevelFields {
            application: from.application.into(),
            transport: from.transport.into(),
            link: from.link.into(),
            physical: from.physical.into(),
        }
        .into()
    }
}

impl From<crate::ffi::AppDecodeLevel> for AppDecodeLevel {
    fn from(from: crate::ffi::AppDecodeLevel) -> Self {
        match from {
            crate::ffi::AppDecodeLevel::Nothing => Self::Nothing,
            crate::ffi::AppDecodeLevel::Header => Self::Header,
            crate::ffi::AppDecodeLevel::ObjectHeaders => Self::ObjectHeaders,
            crate::ffi::AppDecodeLevel::ObjectValues => Self::ObjectValues,
        }
    }
}

impl From<crate::ffi::TransportDecodeLevel> for TransportDecodeLevel {
    fn from(from: crate::ffi::TransportDecodeLevel) -> Self {
        match from {
            crate::ffi::TransportDecodeLevel::Nothing => Self::Nothing,
            crate::ffi::TransportDecodeLevel::Header => Self::Header,
            crate::ffi::TransportDecodeLevel::Payload => Self::Payload,
        }
    }
}

impl From<crate::ffi::LinkDecodeLevel> for LinkDecodeLevel {
    fn from(from: crate::ffi::LinkDecodeLevel) -> Self {
        match from {
            crate::ffi::LinkDecodeLevel::Nothing => Self::Nothing,
            crate::ffi::LinkDecodeLevel::Header => Self::Header,
            crate::ffi::LinkDecodeLevel::Payload => Self::Payload,
        }
    }
}

impl From<crate::ffi::PhysDecodeLevel> for PhysDecodeLevel {
    fn from(from: crate::ffi::PhysDecodeLevel) -> Self {
        match from {
            crate::ffi::PhysDecodeLevel::Nothing => Self::Nothing,
            crate::ffi::PhysDecodeLevel::Length => Self::Length,
            crate::ffi::PhysDecodeLevel::Data => Self::Data,
        }
    }
}

impl From<AppDecodeLevel> for crate::ffi::AppDecodeLevel {
    fn from(from: AppDecodeLevel) -> Self {
        match from {
            AppDecodeLevel::Nothing => crate::ffi::AppDecodeLevel::Nothing,
            AppDecodeLevel::Header => crate::ffi::AppDecodeLevel::Header,
            AppDecodeLevel::ObjectHeaders => crate::ffi::AppDecodeLevel::ObjectHeaders,
            AppDecodeLevel::ObjectValues => crate::ffi::AppDecodeLevel::ObjectValues,
        }
    }
}

impl From<TransportDecodeLevel> for crate::ffi::TransportDecodeLevel {
    fn from(from: TransportDecodeLevel) -> Self {
        match from {
            TransportDecodeLevel::Nothing => crate::ffi::TransportDecodeLevel::Nothing,
            TransportDecodeLevel::Header => crate::ffi::TransportDecodeLevel::Header,
            TransportDecodeLevel::Payload => crate::ffi::TransportDecodeLevel::Payload,
        }
    }
}

impl From<LinkDecodeLevel> for crate::ffi::LinkDecodeLevel {
    fn from(from: LinkDecodeLevel) -> Self {
        match from {
            LinkDecodeLevel::Nothing => crate::ffi::LinkDecodeLevel::Nothing,
            LinkDecodeLevel::Header => crate::ffi::LinkDecodeLevel::Header,
            LinkDecodeLevel::Payload => crate::ffi::LinkDecodeLevel::Payload,
        }
    }
}

impl From<PhysDecodeLevel> for crate::ffi::PhysDecodeLevel {
    fn from(from: PhysDecodeLevel) -> Self {
        match from {
            PhysDecodeLevel::Nothing => crate::ffi::PhysDecodeLevel::Nothing,
            PhysDecodeLevel::Length => crate::ffi::PhysDecodeLevel::Length,
            PhysDecodeLevel::Data => crate::ffi::PhysDecodeLevel::Data,
        }
    }
}
