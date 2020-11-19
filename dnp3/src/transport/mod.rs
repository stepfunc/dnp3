use crate::app::parse::parser::{ParsedFragment, Request, Response};
use crate::app::parse::DecodeLogLevel;
use crate::link::error::LinkError;
use crate::link::header::AddressPair;
use tokio::io::{AsyncRead, AsyncWrite};

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

#[derive(Copy, Clone, Debug)]
pub(crate) enum TransportType {
    Master,
    Outstation,
}

impl TransportType {
    pub(crate) fn is_master(&self) -> bool {
        match self {
            TransportType::Master => true,
            TransportType::Outstation => false,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Fragment<'a> {
    pub(crate) address: AddressPair,
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

impl TransportReader {
    fn new(tt: TransportType, address: u16) -> Self {
        Self {
            logged: false,
            inner: ReaderType::new(tt, address),
        }
    }

    #[cfg(test)]
    pub(crate) fn get_inner(&self) -> &ReaderType {
        &self.inner
    }

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.logged = false;
        self.inner.read(io).await
    }

    pub(crate) fn reset(&mut self) {
        self.inner.reset()
    }

    pub(crate) fn get_response(&mut self, level: DecodeLogLevel) -> Option<(u16, Response)> {
        let (log, address, parsed) = self.peek(level)?;
        match parsed.to_response() {
            Err(err) => {
                if log {
                    log::error!("response error: {}", err)
                }
                None
            }
            Ok(response) => Some((address.source, response)),
        }
    }

    pub(crate) fn get_request(&mut self, level: DecodeLogLevel) -> Option<(AddressPair, Request)> {
        let (log, address, parsed) = self.peek(level)?;
        match parsed.to_request() {
            Err(err) => {
                if log {
                    log::error!("request error: {}", err)
                }
                None
            }
            Ok(request) => Some((address, request)),
        }
    }

    fn peek(&mut self, level: DecodeLogLevel) -> Option<(bool, AddressPair, ParsedFragment)> {
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
        Some((log_this_peek, fragment.address, parsed))
    }
}

pub(crate) fn create_transport_layer(
    tt: TransportType,
    address: u16,
) -> (TransportReader, TransportWriter) {
    (
        TransportReader::new(tt, address),
        TransportWriter::new(tt, address),
    )
}
