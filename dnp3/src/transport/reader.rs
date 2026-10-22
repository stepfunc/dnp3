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
    pub(crate) fn new(parse_options: ParseOptions, inner: T::Reader) -> Self {
        Self {
            parse_options,
            retained: false,
            inner,
        }
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
                    err.into(parsed.control.seq),
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::link::header::{FrameInfo, FrameType};
    use crate::link::reader::LinkModes;
    use crate::link::LinkRole;
    use crate::transport::mock::MockTransport;
    use crate::util::phys::PhysAddr;

    /// READ with no object headers
    const READ: [u8; 2] = [0xC0, 0x01];

    /// Counts the events logged at INFO, the level of a received fragment
    struct InfoEvents(Arc<AtomicUsize>);

    impl tracing::Subscriber for InfoEvents {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::INFO {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }

        fn enter(&self, _: &tracing::span::Id) {}

        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[tokio::test]
    async fn a_retained_request_is_logged_once() {
        let (io, mut handle) = sfio_tokio_mock_io::mock();
        let mut io = PhysLayer::mock(io);
        let (inner, _) = MockTransport::new(
            LinkModes::test(),
            LinkRole::Master,
            EndpointAddress::raw(1),
            64,
        );
        let mut reader = TransportReader::<MockTransport>::new(ParseOptions::default(), inner);
        reader.set_rx_frame_info(FrameInfo::new(
            EndpointAddress::raw(1),
            None,
            FrameType::Data,
            PhysAddr::None,
        ));
        let count = Arc::new(AtomicUsize::new(0));
        let _subscriber = tracing::subscriber::set_default(InfoEvents(count.clone()));

        handle.read(&READ);
        reader.read(&mut io, DecodeLevel::nothing()).await.unwrap();
        let mut guard = reader.pop_request(None, AppDecodeLevel::Header);
        assert!(matches!(guard.get(), Some(TransportRequest::Request(..))));
        guard.retain();
        drop(guard);
        let guard = reader.pop_request(None, AppDecodeLevel::Header);
        assert!(matches!(guard.get(), Some(TransportRequest::Request(..))));
        drop(guard);
        assert_eq!(count.load(Ordering::Relaxed), 1);

        // the next request is logged
        handle.read(&READ);
        reader.read(&mut io, DecodeLevel::nothing()).await.unwrap();
        let guard = reader.pop_request(None, AppDecodeLevel::Header);
        assert!(matches!(guard.get(), Some(TransportRequest::Request(..))));
        drop(guard);
        assert_eq!(count.load(Ordering::Relaxed), 2);
    }
}
