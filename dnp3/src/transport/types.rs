use crate::entry::EndpointAddress;
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
