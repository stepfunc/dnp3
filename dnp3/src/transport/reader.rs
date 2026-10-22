use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::ParsedFragment;
use crate::app::HeaderParseError;
use crate::decode::{AppDecodeLevel, DecodeLevel};
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::transport::{
    RealTransport, Transport, TransportData, TransportRead, TransportRequest, TransportResponse,
};
use crate::util::phys::PhysLayer;

pub(crate) struct TransportReader<T: Transport> {
    parse_options: ParseOptions,
    /// a guard retained the request at the head, which it already logged
    retained: bool,
    inner: T::Reader,
}

pub(crate) struct RequestGuard<'a, T: Transport> {
    canceled: bool,
    master_address: Option<EndpointAddress>,
    level: AppDecodeLevel,
    reader: &'a mut TransportReader<T>,
}

impl<'a, T: Transport> RequestGuard<'a, T> {
    fn new(
        reader: &'a mut TransportReader<T>,
        master_address: Option<EndpointAddress>,
        level: AppDecodeLevel,
    ) -> Self {
        RequestGuard {
            canceled: false,
            master_address,
            level,
            reader,
        }
    }

    pub(crate) fn retain(&mut self) {
        self.canceled = true;
        self.reader.retained = true;
    }

    /// Parse the available request, or `None` if there is none or it came from another master
    pub(crate) fn get(&self) -> Option<TransportRequest<'_>> {
        self.reader.peek_request(self.master_address, self.level)
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
    pub(crate) fn seed_link(&mut self, seed_data: &[u8]) -> Result<(), LinkError> {
        self.inner.seed_link(seed_data)
    }
}

#[cfg(test)]
impl TransportReader<crate::transport::mock::MockTransport> {
    pub(crate) fn set_rx_frame_info(&mut self, info: crate::link::header::FrameInfo) {
        self.inner.set_rx_frame_info(info)
    }

    pub(crate) fn rx_frame_info(&self) -> crate::transport::mock::reader::RxFrameInfo {
        self.inner.rx_frame_info()
    }
}

impl<T: Transport> TransportReader<T> {
    pub(crate) fn new(inner: T::Reader) -> Self {
        Self {
            parse_options: ParseOptions::get_static(),
            retained: false,
            inner,
        }
    }

    /// Parse with `options` instead of those read when the reader was created
    pub(crate) fn set_parse_options(&mut self, options: ParseOptions) {
        self.parse_options = options;
    }

    pub(crate) async fn read(
        &mut self,
        io: &mut PhysLayer,
        decode_level: DecodeLevel,
    ) -> Result<(), LinkError> {
        self.inner.read(io, decode_level).await
    }

    pub(crate) fn reset(&mut self) {
        self.retained = false;
        self.inner.reset()
    }

    fn pop(&mut self) {
        self.retained = false;
        self.inner.pop();
    }

    pub(crate) fn pop_response(&mut self, level: AppDecodeLevel) -> Option<TransportResponse<'_>> {
        let options = self.parse_options;
        let fragment = match self.inner.pop()? {
            TransportData::Fragment(fragment) => fragment,
            TransportData::LinkLayerMessage(msg) => {
                return Some(TransportResponse::LinkLayerMessage(msg))
            }
        };

        let source = fragment.info.addr;
        let response = match parse(options, level, fragment.data) {
            Ok(parsed) => match parsed.to_response() {
                Ok(response) => TransportResponse::Response(source, response),
                Err(err) => TransportResponse::Error(source, err.into()),
            },
            Err(err) => TransportResponse::Error(source, err.into()),
        };
        Some(response)
    }

    /// Guard the available request, which is consumed when the guard drops unless retained. A
    /// retained request was logged when it was first parsed, so it isn't logged again
    pub(crate) fn pop_request(
        &mut self,
        master_address: Option<EndpointAddress>,
        level: AppDecodeLevel,
    ) -> RequestGuard<'_, T> {
        let level = if self.retained {
            AppDecodeLevel::Nothing
        } else {
            level
        };
        RequestGuard::new(self, master_address, level)
    }

    fn peek_request(
        &self,
        master_address: Option<EndpointAddress>,
        level: AppDecodeLevel,
    ) -> Option<TransportRequest<'_>> {
        let fragment = match self.inner.peek()? {
            TransportData::Fragment(fragment) => fragment,
            TransportData::LinkLayerMessage(_) => return Some(TransportRequest::LinkLayerMessage),
        };

        let parsed = parse(self.parse_options, level, fragment.data);

        if let Some(required_master_addr) = master_address {
            if fragment.info.addr.link != required_master_addr {
                tracing::warn!(
                    "Discarding ASDU from master address: {} (configured address == {})",
                    fragment.info.addr.link.raw_value(),
                    required_master_addr.raw_value()
                );
                return None;
            }
        }

        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(err) => return Some(TransportRequest::Error(fragment.info.addr, err.into())),
        };

        let request = match parsed.to_request() {
            Ok(request) => request,
            Err(err) => {
                return Some(TransportRequest::Error(
                    fragment.info.addr,
                    err.with_seq(parsed.control.seq),
                ))
            }
        };

        Some(TransportRequest::Request(fragment.info, request))
    }
}

/// Parse a received fragment, logging it at `level`
fn parse(
    options: ParseOptions,
    level: AppDecodeLevel,
    fragment: &[u8],
) -> Result<ParsedFragment<'_>, HeaderParseError> {
    let parsed = ParsedFragment::parse(options, fragment);
    if level.enabled() {
        match &parsed {
            Ok(fragment) => tracing::info!("APP RX - {}", fragment.display(level)),
            Err(err) => tracing::warn!("error parsing fragment header: {}", err),
        }
    }
    parsed
}
