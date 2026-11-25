// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::ParsedFragment;
use crate::app::HeaderParseError;
use crate::decode::{AppDecodeLevel, DecodeLevel};
use crate::link::error::LinkError;
use crate::phys::PhysLayer;
use crate::transport::{RealTransport, Transport, TransportData, TransportRead, TransportResponse};

pub(crate) struct TransportReader<T: Transport> {
    parse_options: ParseOptions,
    inner: T::Reader,
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
        self.inner.reset()
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
        let response = match parse_fragment(options, level, fragment.data) {
            Ok(parsed) => match parsed.to_response() {
                Ok(response) => TransportResponse::Response(source, response),
                Err(err) => TransportResponse::Error(source, err.into()),
            },
            Err(err) => TransportResponse::Error(source, err.into()),
        };
        Some(response)
    }
}

/// Parse a received fragment, logging it at `level`
pub(crate) fn parse_fragment(
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
