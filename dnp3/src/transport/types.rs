use crate::app::parse::parser::{Request, Response};
use crate::config::EndpointAddress;
use crate::link::header::BroadcastConfirmMode;

#[derive(Debug, Copy, Clone)]
pub(crate) struct FragmentInfo {
    pub(crate) id: u32,
    pub(crate) source: EndpointAddress,
    pub(crate) broadcast: Option<BroadcastConfirmMode>,
}

impl FragmentInfo {
    pub(crate) fn new(
        id: u32,
        source: EndpointAddress,
        broadcast: Option<BroadcastConfirmMode>,
    ) -> Self {
        FragmentInfo {
            id,
            source,
            broadcast,
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
}

pub(crate) enum TransportRequest<'a> {
    Request(FragmentInfo, Request<'a>),
    LinkLayerMessage(LinkLayerMessage),
}
