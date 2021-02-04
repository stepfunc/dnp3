use crate::app::parse::parser::{DecodeSettings, ParsedFragment};
use crate::app::parse::DecodeLogLevel;
use crate::config::LinkErrorMode;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::outstation::config::Feature;
use crate::tokio::io::{AsyncRead, AsyncWrite};
use crate::transport::{
    FragmentInfo, LinkLayerMessage, TransportData, TransportRequest, TransportResponse,
};

#[cfg(not(test))]
/// This type definition is used so that we can mock the transport reader during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub(crate) type InnerReaderType = crate::transport::real::reader::Reader;
#[cfg(test)]
pub(crate) type InnerReaderType = crate::transport::mock::reader::MockReader;

pub(crate) struct TransportReader {
    is_master: bool,
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
        link_error_mode: LinkErrorMode,
        address: EndpointAddress,
        rx_buffer_size: usize,
    ) -> Self {
        Self {
            is_master: true,
            inner: InnerReaderType::master(link_error_mode, address, rx_buffer_size),
        }
    }

    pub(crate) fn outstation(
        link_error_mode: LinkErrorMode,
        address: EndpointAddress,
        self_address: Feature,
        rx_buffer_size: usize,
    ) -> Self {
        Self {
            is_master: false,
            inner: InnerReaderType::outstation(
                link_error_mode,
                address,
                self_address,
                rx_buffer_size,
            ),
        }
    }

    #[cfg(test)]
    pub(crate) fn get_inner(&mut self) -> &mut InnerReaderType {
        &mut self.inner
    }

    pub(crate) async fn read<T>(
        &mut self,
        io: &mut T,
        level: DecodeLogLevel,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.inner.read(io).await?;
        if level.enabled() {
            self.decode(level);
        }
        Ok(())
    }

    fn decode(&self, level: DecodeLogLevel) {
        if let Some(TransportData::Fragment(fragment)) = self.inner.peek() {
            match ParsedFragment::parse(level.receive(), fragment.data) {
                Ok(fragment) => {
                    if let Err(err) = fragment.objects {
                        tracing::warn!("error parsing object header: {}", err);
                    }

                    if self.is_master {
                        if let Err(err) = fragment.to_response() {
                            tracing::warn!("bad response: {}", err);
                        }
                    } else if let Err(err) = fragment.to_request() {
                        tracing::warn!("bad request: {}", err);
                    }
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
            ParsedTransportData::Fragment(info, fragment) => fragment
                .to_response()
                .ok()
                .map(|response| TransportResponse::Response(info.source, response)),
            ParsedTransportData::LinkLayerMessage(msg) => {
                Some(TransportResponse::LinkLayerMessage(msg))
            }
        }
    }

    pub(crate) fn pop_request(&mut self) -> RequestGuard<'_> {
        RequestGuard::new(self)
    }

    fn peek_request(&mut self) -> Option<TransportRequest> {
        let data = self.parse(true)?;
        match data {
            ParsedTransportData::Fragment(info, fragment) => fragment
                .to_request()
                .ok()
                .map(|request| TransportRequest::Request(info, request)),
            ParsedTransportData::LinkLayerMessage(msg) => {
                Some(TransportRequest::LinkLayerMessage(msg))
            }
        }
    }

    fn parse(&mut self, peek: bool) -> Option<ParsedTransportData> {
        let transport_data = if peek {
            self.inner.peek()?
        } else {
            self.inner.pop()?
        };

        match transport_data {
            TransportData::Fragment(fragment) => {
                let parsed = ParsedFragment::parse(DecodeSettings::none(), fragment.data).ok()?;
                Some(ParsedTransportData::Fragment(fragment.info, parsed))
            }
            TransportData::LinkLayerMessage(msg) => {
                Some(ParsedTransportData::LinkLayerMessage(msg))
            }
        }
    }
}

enum ParsedTransportData<'a> {
    Fragment(FragmentInfo, ParsedFragment<'a>),
    LinkLayerMessage(LinkLayerMessage),
}
