use crate::app::parse::parser::{Request, Response};
use crate::app::{HeaderParseError, RequestValidationError, ResponseValidationError, Sequence};
use crate::link::header::BroadcastConfirmMode;
use crate::link::EndpointAddress;
use crate::util::phys::PhysAddr;

#[derive(Debug, Copy, Clone)]
pub(crate) struct FragmentInfo {
    pub(crate) id: u32,
    pub(crate) source: EndpointAddress,
    pub(crate) broadcast: Option<BroadcastConfirmMode>,
    pub(crate) phys_addr: PhysAddr,
}

impl FragmentInfo {
    pub(crate) fn new(
        id: u32,
        source: EndpointAddress,
        broadcast: Option<BroadcastConfirmMode>,
        phys_addr: PhysAddr,
    ) -> Self {
        FragmentInfo {
            id,
            source,
            broadcast,
            phys_addr,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Fragment<'a> {
    pub(crate) info: FragmentInfo,
    pub(crate) data: &'a [u8],
}

#[derive(Debug)]
pub(crate) enum TransportData<'a> {
    Fragment(Fragment<'a>),
    LinkLayerMessage(LinkLayerMessage),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct LinkLayerMessage {
    pub(crate) source: EndpointAddress,
    pub(crate) message: LinkLayerMessageType,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum LinkLayerMessageType {
    LinkStatusRequest,
    LinkStatusResponse,
}

pub(crate) enum TransportResponse<'a> {
    Response(EndpointAddress, Response<'a>),
    LinkLayerMessage(LinkLayerMessage),
    Error(TransportResponseError),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum TransportResponseError {
    HeaderParseError(HeaderParseError),
    ResponseValidationError(ResponseValidationError),
}

impl From<HeaderParseError> for TransportResponseError {
    fn from(from: HeaderParseError) -> Self {
        Self::HeaderParseError(from)
    }
}

impl From<ResponseValidationError> for TransportResponseError {
    fn from(from: ResponseValidationError) -> Self {
        Self::ResponseValidationError(from)
    }
}

pub(crate) enum TransportRequest<'a> {
    Request(FragmentInfo, Request<'a>),
    LinkLayerMessage,
    Error(EndpointAddress, TransportRequestError),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum TransportRequestError {
    HeaderParseError(HeaderParseError),
    RequestValidationError(Sequence, RequestValidationError),
}

impl RequestValidationError {
    pub(crate) fn into(self, seq: Sequence) -> TransportRequestError {
        TransportRequestError::RequestValidationError(seq, self)
    }
}

impl From<HeaderParseError> for TransportRequestError {
    fn from(from: HeaderParseError) -> Self {
        Self::HeaderParseError(from)
    }
}
