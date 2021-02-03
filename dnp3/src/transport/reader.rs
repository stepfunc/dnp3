use crate::app::parse::parser::ParsedFragment;
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
    logged: bool,
    inner: InnerReaderType,
}

pub(crate) struct RequestGuard<'a> {
    canceled: bool,
    level: DecodeLogLevel,
    reader: &'a mut TransportReader,
}

impl<'a> RequestGuard<'a> {
    fn new(level: DecodeLogLevel, reader: &'a mut TransportReader) -> Self {
        RequestGuard {
            canceled: false,
            level,
            reader,
        }
    }

    pub(crate) fn retain(&mut self) {
        self.canceled = true
    }

    pub(crate) fn get(&mut self) -> Option<TransportRequest> {
        self.reader.peek_request(self.level)
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
            logged: false,
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
            logged: false,
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

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.inner.read(io).await
    }

    pub(crate) fn reset(&mut self) {
        self.inner.reset()
    }

    fn pop(&mut self) {
        self.inner.pop();
        self.logged = false;
    }

    fn log_fragment(&mut self) -> bool {
        let log_current_fragment = !self.logged;
        self.logged = true;
        log_current_fragment
    }

    pub(crate) fn pop_response(&mut self, level: DecodeLogLevel) -> Option<TransportResponse> {
        let data = self.parse(false, true, level)?;

        match data {
            ParsedTransportData::Fragment(info, fragment) => match fragment.to_response() {
                Err(err) => {
                    tracing::error!("response error: {}", err);
                    None
                }
                Ok(response) => Some(TransportResponse::Response(info.source, response)),
            },
            ParsedTransportData::LinkLayerMessage(msg) => {
                Some(TransportResponse::LinkLayerMessage(msg))
            }
        }
    }

    pub(crate) fn pop_request(&mut self, level: DecodeLogLevel) -> RequestGuard<'_> {
        RequestGuard::new(level, self)
    }

    fn peek_request(&mut self, level: DecodeLogLevel) -> Option<TransportRequest> {
        let log = self.log_fragment();
        let data = self.parse(true, log, level)?;

        match data {
            ParsedTransportData::Fragment(info, fragment) => match fragment.to_request() {
                Err(err) => {
                    if log {
                        tracing::error!("request error: {}", err);
                    }
                    None
                }
                Ok(request) => Some(TransportRequest::Request(info, request)),
            },
            ParsedTransportData::LinkLayerMessage(msg) => {
                Some(TransportRequest::LinkLayerMessage(msg))
            }
        }
    }

    fn parse(
        &mut self,
        peek: bool,
        log: bool,
        level: DecodeLogLevel,
    ) -> Option<ParsedTransportData> {
        let transport_data = if peek {
            self.inner.peek()?
        } else {
            self.inner.pop()?
        };

        match transport_data {
            TransportData::Fragment(fragment) => {
                let level = if log { level } else { DecodeLogLevel::Nothing };
                let parsed = ParsedFragment::parse(level.receive(), fragment.data).ok()?;
                if let Err(err) = parsed.objects {
                    if log {
                        tracing::warn!("error parsing object headers: {}", err);
                    }
                }
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
