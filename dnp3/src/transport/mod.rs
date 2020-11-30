use crate::app::parse::parser::{ParsedFragment, Request, Response};
use crate::app::parse::DecodeLogLevel;
use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::SelfAddressSupport;
use crate::tokio::io::{AsyncRead, AsyncWrite};

#[cfg(not(test))]
/// This type definition is used so that we can mock the transport reader during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub(crate) type ReaderType = crate::transport::reader::Reader;
#[cfg(not(test))]
/// This type definition is used so that we can mock the transport writer during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub(crate) type TransportWriter = crate::transport::writer::Writer;

#[cfg(test)]
pub(crate) mod mocks;
#[cfg(test)]
pub(crate) type ReaderType = crate::transport::mocks::MockReader;
#[cfg(test)]
pub(crate) type TransportWriter = crate::transport::mocks::MockWriter;

pub(crate) mod assembler;
pub(crate) mod header;
pub(crate) mod reader;
pub(crate) mod sequence;
pub(crate) mod writer;

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

pub(crate) mod constants {
    pub(crate) const FIN_MASK: u8 = 0b1000_0000;
    pub(crate) const FIR_MASK: u8 = 0b0100_0000;
}

pub(crate) struct TransportReader {
    logged: bool,
    inner: ReaderType,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum Timeout {
    Yes,
    No,
}

impl TransportReader {
    fn master(address: EndpointAddress) -> Self {
        Self {
            logged: false,
            inner: ReaderType::master(address),
        }
    }

    fn outstation(address: EndpointAddress, self_address_support: SelfAddressSupport) -> Self {
        Self {
            logged: false,
            inner: ReaderType::outstation(address, self_address_support),
        }
    }

    #[cfg(test)]
    pub(crate) fn get_inner(&mut self) -> &mut ReaderType {
        &mut self.inner
    }

    pub(crate) async fn read_next<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.logged = false;
        self.inner.read_next(io).await
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
            res = self.read_next(io) => {
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

    pub(crate) fn get_response(
        &mut self,
        level: DecodeLogLevel,
    ) -> Option<(EndpointAddress, Response)> {
        let (log, info, parsed) = self.peek(level)?;
        match parsed.to_response() {
            Err(err) => {
                if log {
                    log::error!("response error: {}", err)
                }
                None
            }
            Ok(response) => Some((info.source, response)),
        }
    }

    pub(crate) fn get_request(&mut self, level: DecodeLogLevel) -> Option<(FragmentInfo, Request)> {
        let (log, info, parsed) = self.peek(level)?;
        match parsed.to_request() {
            Err(err) => {
                if log {
                    log::error!("request error: {}", err)
                }
                None
            }
            Ok(request) => Some((info, request)),
        }
    }

    fn peek(&mut self, level: DecodeLogLevel) -> Option<(bool, FragmentInfo, ParsedFragment)> {
        let log_this_peek = !self.logged;
        self.logged = true;
        let fragment: Fragment = self.inner.peek()?;
        let level = if log_this_peek {
            level
        } else {
            DecodeLogLevel::Nothing
        };
        let parsed = match ParsedFragment::parse(level.receive(), fragment.data) {
            Err(err) => {
                if log_this_peek {
                    log::warn!("error parsing fragment header: {}", err)
                }
                return None;
            }
            Ok(parsed) => parsed,
        };
        if let Err(err) = parsed.objects {
            if log_this_peek {
                log::warn!("error parsing object headers: {}", err)
            }
        }
        Some((log_this_peek, fragment.info, parsed))
    }
}

pub(crate) fn create_master_transport_layer(
    address: EndpointAddress,
) -> (TransportReader, TransportWriter) {
    (
        TransportReader::master(address),
        TransportWriter::new(EndpointType::Master, address),
    )
}

pub(crate) fn create_outstation_transport_layer(
    address: EndpointAddress,
    self_address_support: SelfAddressSupport,
) -> (TransportReader, TransportWriter) {
    (
        TransportReader::outstation(address, self_address_support),
        TransportWriter::new(EndpointType::Outstation, address),
    )
}
