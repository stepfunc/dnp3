use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::ParsedFragment;
use crate::app::HeaderParseError;
use crate::decode::{AppDecodeLevel, DecodeLevel};
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::transport::{
    FragmentAddr, FragmentInfo, LinkLayerMessage, RealTransport, Transport, TransportData,
    TransportRead, TransportRequest, TransportResponse,
};
use crate::util::phys::PhysLayer;

pub(crate) struct TransportReader<T: Transport> {
    parse_options: ParseOptions,
    inner: T::Reader,
}

pub(crate) struct RequestGuard<'a, T: Transport> {
    canceled: bool,
    reader: &'a mut TransportReader<T>,
}

impl<'a, T: Transport> RequestGuard<'a, T> {
    fn new(reader: &'a mut TransportReader<T>) -> Self {
        RequestGuard {
            canceled: false,
            reader,
        }
    }

    pub(crate) fn retain(&mut self) {
        self.canceled = true
    }

    pub(crate) fn get(&mut self) -> Option<TransportRequest<'_>> {
        self.reader.peek_request()
    }
}

impl<T: Transport> Drop for RequestGuard<'_, T> {
    fn drop(&mut self) {
        if !self.canceled {
            self.reader.pop()
        }
    }
}

impl TransportReader<RealTransport> {
    pub(crate) fn seed_link(&mut self, seed_data: &[u8]) -> Result<(), scursor::WriteError> {
        self.inner.seed_link(seed_data)
    }
}

#[cfg(test)]
impl TransportReader<crate::transport::mock::MockTransport> {
    pub(crate) fn set_rx_frame_info(&mut self, info: crate::link::header::FrameInfo) {
        self.inner.set_rx_frame_info(info)
    }
}

impl<T: Transport> TransportReader<T> {
    pub(crate) fn new(parse_options: ParseOptions, inner: T::Reader) -> Self {
        Self {
            parse_options,
            inner,
        }
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
            match ParsedFragment::parse(self.parse_options, fragment.data) {
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

    pub(crate) fn pop_response(&mut self) -> Option<TransportResponse<'_>> {
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
    ) -> RequestGuard<'_, T> {
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

    fn peek_request(&mut self) -> Option<TransportRequest<'_>> {
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
    ) -> Option<Result<ParsedTransportData<'_>, (HeaderParseError, FragmentAddr)>> {
        let transport_data = if peek {
            self.inner.peek()?
        } else {
            self.inner.pop()?
        };

        match transport_data {
            TransportData::Fragment(fragment) => Some(
                ParsedFragment::parse(self.parse_options, fragment.data)
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
