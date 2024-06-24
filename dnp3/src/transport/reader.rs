use crate::app::parse::parser::ParsedFragment;
use crate::app::HeaderParseError;
use crate::decode::{AppDecodeLevel, DecodeLevel};
use crate::link::error::LinkError;
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::Feature;
use crate::transport::{
    FragmentAddr, FragmentInfo, LinkLayerMessage, TransportData, TransportRequest,
    TransportResponse,
};
use crate::util::phys::PhysLayer;

#[cfg(not(test))]
/// This type definition is used so that we can mock the transport reader during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub(crate) type InnerReaderType = crate::transport::real::reader::Reader;
#[cfg(test)]
pub(crate) type InnerReaderType = crate::transport::mock::reader::MockReader;

pub(crate) struct TransportReader {
    inner: InnerReaderType,
}

pub(crate) struct RequestGuard<'a> {
    canceled: bool,
    reader: &'a mut TransportReader,
}

impl<'a> RequestGuard<'a> {
    fn new(reader: &'a mut TransportReader) -> Self {
        RequestGuard {
            canceled: false,
            reader,
        }
    }

    pub(crate) fn retain(&mut self) {
        self.canceled = true
    }

    pub(crate) fn get(&mut self) -> Option<TransportRequest> {
        self.reader.peek_request()
    }
}

impl<'a> Drop for RequestGuard<'a> {
    fn drop(&mut self) {
        if !self.canceled {
            self.reader.pop()
        }
    }
}

impl TransportReader {
    pub(crate) fn master(
        link_modes: LinkModes,
        address: EndpointAddress,
        rx_buffer_size: usize,
    ) -> Self {
        Self {
            inner: InnerReaderType::master(link_modes, address, rx_buffer_size),
        }
    }

    pub(crate) fn outstation(
        link_modes: LinkModes,
        address: EndpointAddress,
        self_address: Feature,
        rx_buffer_size: usize,
    ) -> Self {
        Self {
            inner: InnerReaderType::outstation(link_modes, address, self_address, rx_buffer_size),
        }
    }

    #[cfg(test)]
    pub(crate) fn get_inner(&mut self) -> &mut InnerReaderType {
        &mut self.inner
    }

    pub(crate) fn seed_link(&mut self, seed_data: &[u8]) -> Result<(), scursor::WriteError> {
        self.inner.seed_link(seed_data)
    }

    pub(crate) async fn read(
        &mut self,
        io: &mut PhysLayer,
        decode_level: DecodeLevel,
    ) -> Result<(), LinkError> {
        self.inner.read(io, decode_level).await?;
        if decode_level.application.enabled() {
            self.decode(decode_level.application);
        }
        Ok(())
    }

    fn decode(&self, level: AppDecodeLevel) {
        if let Some(TransportData::Fragment(fragment)) = self.inner.peek() {
            match ParsedFragment::parse(fragment.data) {
                Ok(fragment) => {
                    tracing::info!("APP RX - {}", fragment.display(level));
                }
                Err(err) => {
                    tracing::warn!("error parsing fragment header: {}", err);
                }
            }
        }
    }

    pub(crate) fn reset(&mut self) {
        self.inner.reset()
    }

    fn pop(&mut self) {
        self.inner.pop();
    }

    pub(crate) fn pop_response(&mut self) -> Option<TransportResponse> {
        let data = self.parse(false)?;

        match data {
            Ok(ParsedTransportData::Fragment(info, fragment)) => match fragment.to_response() {
                Ok(response) => Some(TransportResponse::Response(info.addr, response)),
                Err(err) => Some(TransportResponse::Error(err.into())),
            },
            Ok(ParsedTransportData::LinkLayerMessage(msg)) => {
                Some(TransportResponse::LinkLayerMessage(msg))
            }
            Err((err, _)) => Some(TransportResponse::Error(err.into())),
        }
    }

    pub(crate) fn pop_request(
        &mut self,
        master_address: Option<EndpointAddress>,
    ) -> RequestGuard<'_> {
        if let Some(TransportRequest::Request(info, _)) = self.peek_request() {
            if let Some(required_master_addr) = master_address {
                if info.addr.link != required_master_addr {
                    tracing::warn!(
                        "Discarding ASDU from master address: {} (configured address == {})",
                        info.addr.link.raw_value(),
                        required_master_addr.raw_value()
                    );
                    self.pop();
                }
            }
        }
        RequestGuard::new(self)
    }

    fn peek_request(&mut self) -> Option<TransportRequest> {
        let data = self.parse(true)?;
        match data {
            Ok(ParsedTransportData::Fragment(info, fragment)) => match fragment.to_request() {
                Ok(request) => Some(TransportRequest::Request(info, request)),
                Err(err) => Some(TransportRequest::Error(
                    info.addr,
                    err.into(fragment.control.seq),
                )),
            },
            Ok(ParsedTransportData::LinkLayerMessage(_)) => {
                Some(TransportRequest::LinkLayerMessage)
            }
            Err((err, source)) => Some(TransportRequest::Error(source, err.into())),
        }
    }

    fn parse(
        &mut self,
        peek: bool,
    ) -> Option<Result<ParsedTransportData, (HeaderParseError, FragmentAddr)>> {
        let transport_data = if peek {
            self.inner.peek()?
        } else {
            self.inner.pop()?
        };

        match transport_data {
            TransportData::Fragment(fragment) => Some(
                ParsedFragment::parse(fragment.data)
                    .map(|parsed| ParsedTransportData::Fragment(fragment.info, parsed))
                    .map_err(|err| (err, fragment.info.addr)),
            ),
            TransportData::LinkLayerMessage(msg) => {
                Some(Ok(ParsedTransportData::LinkLayerMessage(msg)))
            }
        }
    }
}

enum ParsedTransportData<'a> {
    Fragment(FragmentInfo, ParsedFragment<'a>),
    LinkLayerMessage(LinkLayerMessage),
}
