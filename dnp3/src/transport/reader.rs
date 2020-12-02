use crate::app::parse::parser::{ParsedFragment, Request, Response};
use crate::app::parse::DecodeLogLevel;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::outstation::SelfAddressSupport;
use crate::tokio::io::{AsyncRead, AsyncWrite};
use crate::transport::{Fragment, FragmentInfo};

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
