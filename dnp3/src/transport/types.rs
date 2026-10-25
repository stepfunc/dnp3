// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::parser::{Request, Response};
use crate::app::{HeaderParseError, RequestValidationError, ResponseValidationError, Sequence};
use crate::link::header::BroadcastConfirmMode;
use crate::link::EndpointAddress;
use crate::phys::PhysAddr;

#[derive(Debug, Copy, Clone)]
pub(crate) struct FragmentAddr {
    /// remote link address
    pub(crate) link: EndpointAddress,
    /// remote physical-layer address
    pub(crate) phys: PhysAddr,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct FragmentInfo {
    pub(crate) addr: FragmentAddr,
    pub(crate) broadcast: Option<BroadcastConfirmMode>,
}

impl FragmentInfo {
    pub(crate) fn new(addr: FragmentAddr, broadcast: Option<BroadcastConfirmMode>) -> Self {
        FragmentInfo { addr, broadcast }
    }
}

pub(crate) struct Fragment<'a> {
    pub(crate) info: FragmentInfo,
    pub(crate) data: &'a [u8],
}

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
    Response(FragmentAddr, Response<'a>),
    LinkLayerMessage(LinkLayerMessage),
    Error(FragmentAddr, TransportResponseError),
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
    Error(FragmentAddr, TransportRequestError),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum TransportRequestError {
    HeaderParseError(HeaderParseError),
    RequestValidationError(Sequence, RequestValidationError),
}

impl RequestValidationError {
    pub(crate) fn with_seq(self, seq: Sequence) -> TransportRequestError {
        TransportRequestError::RequestValidationError(seq, self)
    }
}

impl From<HeaderParseError> for TransportRequestError {
    fn from(from: HeaderParseError) -> Self {
        Self::HeaderParseError(from)
    }
}
