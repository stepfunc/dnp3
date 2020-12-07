use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::DecodeLogLevel;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::outstation::SelfAddressSupport;
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum Timeout {
    Yes,
    No,
}

impl TransportReader {
    pub(crate) fn master(address: EndpointAddress, rx_buffer_size: usize) -> Self {
        Self {
            logged: false,
            inner: InnerReaderType::master(address, rx_buffer_size),
        }
    }

    pub(crate) fn outstation(
        address: EndpointAddress,
        self_address_support: SelfAddressSupport,
        rx_buffer_size: usize,
    ) -> Self {
        Self {
            logged: false,
            inner: InnerReaderType::outstation(address, self_address_support, rx_buffer_size),
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

    pub(crate) async fn read_with_timeout<T>(
        &mut self,
        io: &mut T,
        deadline: crate::tokio::time::Instant,
    ) -> Result<Timeout, LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        crate::tokio::select! {
            res = self.read(io) => {
                res?;
                Ok(Timeout::No)
            },
            _ = crate::tokio::time::delay_until(deadline) => {
                Ok(Timeout::Yes)
            }
        }
    }

    pub(crate) fn reset(&mut self) {
        self.inner.reset()
    }

    pub(crate) fn pop(&mut self) {
        self.inner.pop();
        self.logged = false;
    }

    fn log_fragment(&mut self) -> bool {
        let log_current_fragment = !self.logged;
        self.logged = true;
        log_current_fragment
    }

    pub(crate) fn pop_response(&mut self, level: DecodeLogLevel) -> Option<TransportResponse> {
        let log = self.log_fragment();
        let data = self.parse(false, log, level)?;

        match data {
            ParsedTransportData::Fragment(info, fragment) => match fragment.to_response() {
                Err(err) => {
                    if log {
                        log::error!("response error: {}", err);
                    }
                    None
                }
                Ok(response) => Some(TransportResponse::Response(info.source, response)),
            },
            ParsedTransportData::LinkLayerMessage(msg) => {
                Some(TransportResponse::LinkLayerMessage(msg))
            }
        }
    }

    pub(crate) fn peek_request(&mut self, level: DecodeLogLevel) -> Option<TransportRequest> {
        let log = self.log_fragment();
        let data = self.parse(true, log, level)?;

        match data {
            ParsedTransportData::Fragment(info, fragment) => match fragment.to_request() {
                Err(err) => {
                    if log {
                        log::error!("request error: {}", err);
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
                        log::warn!("error parsing object headers: {}", err);
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
