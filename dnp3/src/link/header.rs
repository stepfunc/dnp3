use crate::link::EndpointAddress;
use crate::util::phys::PhysAddr;

use super::function::Function;

pub(crate) mod constants {
    pub(crate) const MASK_DIR: u8 = 0x80;
    pub(crate) const MASK_PRM: u8 = 0x40;
    pub(crate) const MASK_FCB: u8 = 0x20;
    pub(crate) const MASK_FCV: u8 = 0x10;
    pub(crate) const MASK_FUNC: u8 = 0x0F;
    pub(crate) const MASK_FUNC_OR_PRM: u8 = MASK_PRM | MASK_FUNC;
    pub(crate) const BROADCAST_CONFIRM_OPTIONAL: u16 = 0xFFFF;
    pub(crate) const BROADCAST_CONFIRM_MANDATORY: u16 = 0xFFFE;
    pub(crate) const BROADCAST_CONFIRM_NOT_REQUIRED: u16 = 0xFFFD;
    pub(crate) const SELF_ADDRESS: u16 = 0xFFFC;
    pub(crate) const RESERVED_START: u16 = 0xFFF0;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum BroadcastConfirmMode {
    Optional,
    Mandatory,
    NotRequired,
}

impl BroadcastConfirmMode {
    pub(crate) fn address(&self) -> u16 {
        match self {
            BroadcastConfirmMode::Optional => constants::BROADCAST_CONFIRM_OPTIONAL,
            BroadcastConfirmMode::Mandatory => constants::BROADCAST_CONFIRM_MANDATORY,
            BroadcastConfirmMode::NotRequired => constants::BROADCAST_CONFIRM_NOT_REQUIRED,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum AnyAddress {
    Reserved(u16),
    Endpoint(EndpointAddress),
    Broadcast(BroadcastConfirmMode),
    SelfAddress,
}

impl AnyAddress {
    pub(crate) const fn from(address: u16) -> Self {
        match address {
            constants::BROADCAST_CONFIRM_OPTIONAL => {
                Self::Broadcast(BroadcastConfirmMode::Optional)
            }
            constants::BROADCAST_CONFIRM_MANDATORY => {
                Self::Broadcast(BroadcastConfirmMode::Mandatory)
            }
            constants::BROADCAST_CONFIRM_NOT_REQUIRED => {
                Self::Broadcast(BroadcastConfirmMode::NotRequired)
            }
            constants::SELF_ADDRESS => Self::SelfAddress,
            // reserved addresses
            x if x >= constants::RESERVED_START => Self::Reserved(x),
            // anything else is an allowed link address for a master or outstation
            _ => Self::Endpoint(EndpointAddress::raw(address)),
        }
    }

    pub(crate) fn value(&self) -> u16 {
        match self {
            Self::Reserved(x) => *x,
            Self::Endpoint(x) => x.raw_value(),
            Self::Broadcast(x) => x.address(),
            Self::SelfAddress => constants::SELF_ADDRESS,
        }
    }
}

impl std::fmt::Display for AnyAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AnyAddress::Reserved(x) => write!(f, "reserved address ({x})"),
            AnyAddress::Endpoint(x) => write!(f, "normal address ({x})"),
            AnyAddress::SelfAddress => write!(f, "self address ({})", constants::SELF_ADDRESS),
            AnyAddress::Broadcast(details) => {
                write!(f, "broadcast address ({})", details.address())
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct ControlField {
    pub(crate) func: Function,
    pub(crate) master: bool,
    pub(crate) fcb: bool,
    pub(crate) fcv: bool,
}

impl ControlField {
    pub(crate) fn new(master: bool, function: Function) -> Self {
        Self {
            func: function,
            master,
            fcb: false,
            fcv: false,
        }
    }

    pub(crate) fn from(byte: u8) -> ControlField {
        ControlField {
            func: Function::from(byte & constants::MASK_FUNC_OR_PRM),
            master: (byte & constants::MASK_DIR) != 0,
            fcb: (byte & constants::MASK_FCB) != 0,
            fcv: (byte & constants::MASK_FCV) != 0,
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
        let mut ret = 0;
        ret |= if self.master { constants::MASK_DIR } else { 0 };
        // the PRM bit is part of the function code
        ret |= if self.fcb { constants::MASK_FCB } else { 0 };
        ret |= if self.fcv { constants::MASK_FCV } else { 0 };
        ret |= self.func.to_u8();
        ret
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct FrameInfo {
    pub(crate) source: EndpointAddress,
    pub(crate) broadcast: Option<BroadcastConfirmMode>,
    pub(crate) frame_type: FrameType,
    pub(crate) phys_addr: PhysAddr,
}

impl FrameInfo {
    pub(crate) fn new(
        source: EndpointAddress,
        broadcast: Option<BroadcastConfirmMode>,
        frame_type: FrameType,
        phys_addr: PhysAddr,
    ) -> Self {
        Self {
            source,
            broadcast,
            frame_type,
            phys_addr,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum FrameType {
    Data,
    LinkStatusRequest,
    LinkStatusResponse,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct Header {
    pub(crate) control: ControlField,
    pub(crate) destination: AnyAddress,
    pub(crate) source: AnyAddress,
}

impl Header {
    pub(crate) fn new(control: ControlField, destination: AnyAddress, source: AnyAddress) -> Self {
        Self {
            control,
            destination,
            source,
        }
    }

    pub(crate) fn unconfirmed_user_data(
        is_master: bool,
        destination: AnyAddress,
        source: AnyAddress,
    ) -> Self {
        Self::new(
            ControlField::new(is_master, Function::PriUnconfirmedUserData),
            destination,
            source,
        )
    }

    pub(crate) fn request_link_status(
        is_master: bool,
        destination: EndpointAddress,
        source: EndpointAddress,
    ) -> Self {
        Self::new(
            ControlField::new(is_master, Function::PriRequestLinkStatus),
            destination.wrap(),
            source.wrap(),
        )
    }
}
