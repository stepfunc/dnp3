// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use xxhash_rust::xxh64::xxh64;

use super::context::SessionContext;
use super::phase::{Answer, SolTx};
use super::response::{Prepared, ResponseSlot};
use super::state::{FragmentType, Intake, LastValidRequest, Received};
use crate::app::parse::parser::{HeaderCollection, Request};
use crate::app::{
    ControlField, FunctionCode, HeaderParseError, Iin, Iin2, ObjectParseError, ResponseFunction,
    ResponseHeader, Sequence,
};
use crate::decode::AppDecodeLevel;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::read_log::Tracing;
use crate::transport::{FragmentAddr, FragmentInfo, TransportData};
use crate::util::clock::MonotonicClock;

impl<C: MonotonicClock> SessionContext<C> {
    /// Filter and classify one incoming fragment or link-layer message. Every accepted fragment or
    /// link-layer message counts as link activity for keep-alive, and every request or unknown
    /// function ends any active control selection (except a retry of the SELECT).
    ///
    /// Returns `None` when there is nothing for the phase to process (link-layer message,
    /// unauthorized master, invalid request, or discarded SELECT), leaving any staged transmission
    /// intact.
    pub(super) fn receive<'a>(
        &mut self,
        input: TransportData<'a>,
        app_decode_level: AppDecodeLevel,
    ) -> Option<Intake<'a>> {
        let fragment = match input {
            TransportData::LinkLayerMessage(_) => {
                self.keep_alive.on_activity(self.clock.now());
                return None;
            }
            TransportData::Fragment(fragment) => fragment,
        };

        if let Some(required_master_addr) = self.required_master_address() {
            if fragment.info.addr.link != required_master_addr {
                tracing::warn!(
                    "Discarding ASDU from master address: {} (configured address == {})",
                    fragment.info.addr.link.raw_value(),
                    required_master_addr.raw_value()
                );
                return None;
            }
        }

        self.keep_alive.on_activity(self.clock.now());

        let parsed = match crate::transport::parse_fragment(
            self.parse_options,
            app_decode_level,
            fragment.data,
        ) {
            Ok(parsed) => parsed,
            Err(HeaderParseError::UnknownFunction(seq, _)) => {
                // Unknown function ends any active selection (TB2016-001).
                self.state.select = None;
                return Some(Intake::UnknownFunction(fragment.info.addr, seq));
            }
            Err(HeaderParseError::InsufficientBytes) => {
                return None;
            }
        };

        let request = match parsed.to_request() {
            Ok(request) => request,
            Err(err) => {
                // IEEE 1815-2012 clause 6 discards a fragment that isn't a valid request.
                tracing::warn!("ignoring invalid request: {err}");
                return None;
            }
        };

        if request.header.function == FunctionCode::Confirm {
            return Some(Intake::Confirm(fragment.info.addr, request.header));
        }

        let classified = self.classify(fragment.info, request)?;
        Some(Intake::Request(Received {
            info: fragment.info,
            header: request.header,
            fragment: classified,
        }))
    }

    /// Classify a request against the last valid request and update selection state.
    ///
    /// Returns `None` if a SELECT repeats the active selection's sequence number with different
    /// contents (IEEE 1815-2012 Table 4-9). Otherwise ends any active selection, passing it to a
    /// `NewNonRead` (for OPERATE) unless the request is a retry of the SELECT itself (4.4.4 Rule 4
    /// and Table 4-9).
    fn classify<'a>(
        &mut self,
        info: FragmentInfo,
        request: Request<'a>,
    ) -> Option<FragmentType<'a>> {
        if let Some(mode) = info.broadcast {
            self.state.select = None;
            return Some(FragmentType::Broadcast {
                mode,
                objects: request.objects,
            });
        }

        let this_hash = xxh64(request.raw_fragment, 0);

        let object_headers = match request.objects {
            Ok(x) => x,
            Err(err) => {
                self.state.select = None;
                return Some(FragmentType::MalformedRequest(this_hash, err));
            }
        };

        let is_repeat = self.state.last_valid_request.is_some_and(|last| {
            last.seq == request.header.control.seq && last.request_hash == this_hash
        });

        if request.header.function == FunctionCode::Read {
            self.state.select = None;
            return Some(if is_repeat {
                FragmentType::RepeatRead(this_hash, object_headers)
            } else {
                FragmentType::NewRead(this_hash, object_headers)
            });
        }

        if is_repeat {
            // Keep the selection only when this repeat is a retry of the SELECT itself.
            if !self
                .state
                .select
                .is_some_and(|select| select.request_hash() == this_hash)
            {
                self.state.select = None;
            }
            return Some(FragmentType::RepeatNonRead);
        }

        if request.header.function == FunctionCode::Select
            && self
                .state
                .select
                .is_some_and(|select| select.seq() == request.header.control.seq)
        {
            tracing::warn!("discarding SELECT that repeats the selection's sequence number with different contents");
            return None;
        }

        let selection = self.state.select.take();
        Some(FragmentType::NewNonRead {
            hash: this_hash,
            objects: object_headers,
            selection,
        })
    }

    /// Defer a READ received during `UnsolConfirmWait` so it runs once the wait ends.
    pub(super) fn defer_read(
        &mut self,
        hash: u64,
        seq: Sequence,
        info: FragmentInfo,
        objects: HeaderCollection<'_>,
    ) {
        self.state
            .deferred_read
            .set::<Tracing>(hash, seq, info, objects);
    }

    /// Execute an accepted request and publish any solicited response into `sol`.
    ///
    /// Always discards any deferred READ (IEEE 1815-2012 Table 6-1 rows 4-9 and 24-29); READs
    /// during `UnsolConfirmWait` are deferred by `Phase` before calling this method.
    pub(super) async fn accept_request(
        &mut self,
        sol: &mut ResponseSlot,
        received: Received<'_>,
    ) -> Answer {
        let Received {
            info,
            header,
            fragment,
        } = received;
        let seq = header.control.seq;

        self.state.deferred_read.clear();

        match fragment {
            FragmentType::NewRead(hash, objects) | FragmentType::RepeatRead(hash, objects) => {
                // IEEE 1815-2012 Table 6-1 (Idle, row 8) says to resend the previous response to a
                // repeated READ. We deliberately answer it as a new READ instead: the fresh
                // response carries current values, and events that were never confirmed are still
                // buffered, so they are reported again.
                let response = format_first_read_response(sol, &mut self.db, seq, objects);
                let tx = self.publish_solicited(info.addr, response);
                self.state.last_valid_request = Some(LastValidRequest::new(seq, hash));
                Answer::Respond(tx)
            }
            FragmentType::MalformedRequest(hash, err) => {
                // IEEE 1815-2012 clause 6.1: a request whose objects don't parse is still the last
                // request, answered with IIN2.
                let header = ResponseHeader::solicited(seq, Iin::default() | Iin2::from(err));
                let tx = self.publish_solicited(info.addr, sol.empty(header));
                self.state.last_valid_request = Some(LastValidRequest::new(seq, hash));
                Answer::Respond(tx)
            }
            FragmentType::NewNonRead {
                hash,
                objects,
                selection,
            } => {
                let last = LastValidRequest::new(seq, hash);
                let result = self
                    .handle_non_read(sol, header.function, last, selection, objects)
                    .await;
                self.state.last_valid_request = Some(last);
                match result {
                    Some(response) => Answer::Respond(self.publish_solicited(info.addr, response)),
                    None => {
                        sol.clear();
                        Answer::Silent
                    }
                }
            }
            FragmentType::RepeatNonRead => {
                // IEEE 1815-2012 Table 6-1: send the same response, byte for byte, which waits for
                // its confirm again if it asked for one.
                match sol.header() {
                    Some(header) => Answer::Respond(SolTx::new(info.addr, header)),
                    None => Answer::Silent,
                }
            }
            FragmentType::Broadcast { mode, objects } => {
                Answer::Broadcast(self.process_broadcast(mode, header, objects).await)
            }
        }
    }

    /// Build an IIN2.0 answer to a request with an unknown function code in `transient`.
    /// Selection was already cleared by [`Self::receive`]; this also discards any deferred READ,
    /// while leaving the last valid request and its response intact (TB2016-001).
    pub(super) fn stage_unknown_function(
        &mut self,
        transient: &mut [u8; ResponseHeader::LENGTH],
        seq: Sequence,
    ) {
        self.state.deferred_read.clear();
        let iin = Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT;
        self.build_transient_solicited(transient, ResponseHeader::solicited(seq, iin));
    }

    /// Format and publish the next continuation fragment of a multi-fragment READ series.
    pub(super) fn next_read_fragment(
        &mut self,
        sol: &mut ResponseSlot,
        to: FragmentAddr,
        seq: Sequence,
    ) -> SolTx {
        let response = format_read_response(sol, &mut self.db, false, seq, Iin2::default());
        self.publish_solicited(to, response)
    }

    /// If a deferred READ is pending, select it and publish its first response fragment into `sol`.
    pub(super) fn stage_deferred_read(&mut self, sol: &mut ResponseSlot) -> Option<SolTx> {
        let deferred = self.state.deferred_read.select(&mut self.db)?;
        tracing::info!("handling deferred READ request");
        let response = format_read_response(sol, &mut self.db, true, deferred.seq, deferred.iin2);
        let tx = self.publish_solicited(deferred.info.addr, response);
        self.state.last_valid_request = Some(LastValidRequest::new(deferred.seq, deferred.hash));
        Some(tx)
    }
}

fn format_first_read_response<'a>(
    sol: &'a mut ResponseSlot,
    database: &mut DatabaseHandle,
    seq: Sequence,
    object_headers: HeaderCollection,
) -> Prepared<'a> {
    let iin2 = database.select(&object_headers);
    format_read_response(sol, database, true, seq, iin2)
}

fn format_read_response<'a>(
    sol: &'a mut ResponseSlot,
    database: &mut DatabaseHandle,
    fir: bool,
    seq: Sequence,
    iin2: Iin2,
) -> Prepared<'a> {
    let (result, staged) = sol.build(|cursor| Ok(database.write_response_headers(cursor)));
    let header = match result {
        Ok(info) if !info.complete && !staged.has_objects() => {
            tracing::error!("unable to make progress writing a READ response");
            database.reset();
            ResponseHeader::new(
                ControlField::response(seq, fir, true, false),
                ResponseFunction::Response,
                Iin::default() | Iin2::PARAMETER_ERROR,
            )
        }
        Ok(info) => ResponseHeader::new(
            ControlField::response(seq, fir, info.complete, info.need_confirm()),
            ResponseFunction::Response,
            Iin::default() | iin2,
        ),
        Err(err) => {
            tracing::error!("unable to write a READ response: {err:?}");
            ResponseHeader::parameter_error(seq)
        }
    };
    staged.with_header(header)
}

impl From<ObjectParseError> for Iin2 {
    fn from(err: ObjectParseError) -> Self {
        match err {
            ObjectParseError::InsufficientBytes => Iin2::PARAMETER_ERROR,
            ObjectParseError::InvalidQualifierForVariation(_, _) => Iin2::PARAMETER_ERROR,
            ObjectParseError::InvalidRange(_, _) => Iin2::PARAMETER_ERROR,
            ObjectParseError::UnknownGroupVariation(_, _) => Iin2::OBJECT_UNKNOWN,
            #[expect(
                deprecated,
                reason = "the mapping must cover every variant until removal"
            )]
            ObjectParseError::UnsupportedQualifierCode(_) => Iin2::PARAMETER_ERROR,
            ObjectParseError::UnknownQualifier(_) => Iin2::PARAMETER_ERROR,
            ObjectParseError::ZeroLengthOctetData => Iin2::PARAMETER_ERROR,
            ObjectParseError::BadAttribute(_) => Iin2::PARAMETER_ERROR,
            ObjectParseError::BadEncoding => Iin2::PARAMETER_ERROR,
            ObjectParseError::UnsupportedFreeFormatCount(_) => Iin2::PARAMETER_ERROR,
        }
    }
}
