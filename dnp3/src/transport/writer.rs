// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::{FragmentDisplay, ParsedFragment};
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::transport::{FragmentAddr, Transport, TransportWrite};
use crate::util::phys::PhysLayer;

pub(crate) struct TransportWriter<T: Transport> {
    inner: T::Writer,
}

impl<T: Transport> TransportWriter<T> {
    pub(crate) fn new(inner: T::Writer) -> Self {
        Self { inner }
    }

    pub(crate) fn reset(&mut self) {
        self.inner.reset()
    }

    pub(crate) async fn write(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        destination: FragmentAddr,
        fragment: &[u8],
    ) -> Result<(), LinkError> {
        if level.application.enabled() {
            match ParsedFragment::parse(ParseOptions::parse_everything(), fragment) {
                Ok(parsed) => {
                    let x: FragmentDisplay = parsed.display(level.application);
                    tracing::info!("APP TX - {}", x);
                }
                Err(err) => {
                    tracing::error!("error decoding transmitted fragment: {err}");
                }
            }
        }
        self.inner.write(io, level, destination, fragment).await
    }

    pub(crate) async fn send_link_status_request(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        dest: FragmentAddr,
    ) -> Result<(), LinkError> {
        self.inner.write_link_status_request(io, dest, level).await
    }
}
