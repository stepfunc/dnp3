// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;
use crate::outstation::read_log::Tracing;

impl OutstationSession {
    /// Take the master's next fragment. This is where every request enters the session, in any
    /// state: any fragment from the master is link activity, and a request is classified once,
    /// against the last valid request and the selection.
    pub(super) fn receive<'a, T: Transport>(
        &mut self,
        guard: &'a RequestGuard<'_, T>,
    ) -> Intake<'a> {
        let (info, request) = match guard.get() {
            None => return Intake::Nothing,
            Some(TransportRequest::LinkLayerMessage) => {
                self.on_link_activity();
                return Intake::Nothing;
            }
            Some(TransportRequest::Error(from, err)) => {
                self.on_link_activity();
                return match Self::error_response_seq(err) {
                    Some(seq) => Intake::UnknownFunction(from, seq),
                    None => Intake::Nothing,
                };
            }
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                (info, request)
            }
        };

        if request.header.function == FunctionCode::Confirm {
            return Intake::Confirm(info.addr, request.header);
        }

        let fragment = self.classify(info, request);
        if self.is_discarded_select(
            &fragment,
            request.header.function,
            request.header.control.seq,
        ) {
            return Intake::Nothing;
        }

        Intake::Request(Received {
            info,
            request,
            fragment,
        })
    }

    pub(super) fn classify<'a>(
        &self,
        info: FragmentInfo,
        request: Request<'a>,
    ) -> FragmentType<'a> {
        if let Some(mode) = info.broadcast {
            return FragmentType::Broadcast(mode);
        }

        // we need to calculate a digest to deduplicate
        let this_hash = xxh64(request.raw_fragment, 0);

        let object_headers = match request.objects {
            Ok(x) => x,
            Err(err) => return FragmentType::MalformedRequest(this_hash, err),
        };

        // detect duplicate requests
        if let Some(last) = self.state.last_valid_request {
            if last.seq == request.header.control.seq && last.request_hash == this_hash {
                return if request.header.function == FunctionCode::Read {
                    FragmentType::RepeatRead(this_hash, object_headers)
                } else {
                    FragmentType::RepeatNonRead(this_hash)
                };
            }
        }

        if request.header.function == FunctionCode::Read {
            FragmentType::NewRead(this_hash, object_headers)
        } else {
            FragmentType::NewNonRead(this_hash, object_headers)
        }
    }

    /// Table 4-9: a SELECT with the selection's sequence number but different contents is discarded
    pub(super) fn is_discarded_select(
        &self,
        fragment: &FragmentType,
        function: FunctionCode,
        seq: Sequence,
    ) -> bool {
        let discarded = matches!(fragment, FragmentType::NewNonRead(_, _))
            && function == FunctionCode::Select
            && self.state.select.is_some_and(|select| select.seq() == seq);
        if discarded {
            tracing::warn!("discarding SELECT that repeats the selection's sequence number with different contents");
        }
        discarded
    }

    /// End the selection, giving it to an OPERATE to check, unless the request is a retry of the
    /// SELECT. IEEE 1815-2012 4.4.4 Rule 4 and Table 4-9: ending a selection early can only make
    /// an OPERATE fail, while keeping one too long could execute a control, so every request ends
    /// it except a retry of the SELECT and a SELECT that it discards. A confirm isn't a request.
    pub(super) fn take_selection(&mut self, fragment: &FragmentType) -> Option<SelectState> {
        if let FragmentType::RepeatNonRead(hash) = fragment {
            if self
                .state
                .select
                .is_some_and(|select| select.request_hash() == *hash)
            {
                return None;
            }
        }
        self.state.select.take()
    }

    /// Accept a request in idle or the unsolicited confirm wait (IEEE 1815-2012 Table 6-1 rows
    /// 4-9 and 24-29): answer it, or defer a READ. A request that isn't a READ discards the
    /// deferred READ. Returns the confirm that the answer waits for, and how a broadcast was
    /// processed.
    pub(super) async fn accept<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        out: &mut Responder<T>,
        database: &mut DatabaseHandle,
        received: Received<'_>,
        selection: Option<SelectState>,
        reads: Reads,
    ) -> Result<Accepted, LinkError> {
        let Received {
            info,
            request,
            fragment,
        } = received;
        let seq = request.header.control.seq;

        if !fragment.is_read() {
            self.state.deferred_read.clear();
        }

        let confirm = match fragment {
            FragmentType::NewRead(hash, objects) | FragmentType::RepeatRead(hash, objects) => {
                match reads {
                    Reads::Answer => {
                        // IEEE 1815-2012 Table 6-1 (Idle, row 8) says to resend the previous
                        // response to a repeated READ. We deliberately answer it as a new READ
                        // instead: the fresh response carries current values, and events that were
                        // never confirmed are still buffered, so they are reported again
                        let response =
                            Self::format_first_read_response(&mut out.sol, database, seq, objects);
                        self.respond(
                            io,
                            &mut out.writer,
                            info.addr,
                            LastValidRequest::new(seq, hash),
                            response,
                            database,
                        )
                        .await
                    }
                    Reads::Defer => {
                        self.state
                            .deferred_read
                            .set::<Tracing>(hash, seq, info, objects);
                        Ok(None)
                    }
                }
            }
            FragmentType::MalformedRequest(hash, err) => {
                // IEEE 1815-2012 clause 6.1: a request whose objects don't parse is still the last
                // request, answered with IIN2
                let header = ResponseHeader::solicited(seq, Iin::default() | Iin2::from(err));
                self.respond(
                    io,
                    &mut out.writer,
                    info.addr,
                    LastValidRequest::new(seq, hash),
                    out.sol.empty(header),
                    database,
                )
                .await
            }
            FragmentType::NewNonRead(hash, objects) => {
                let last = LastValidRequest::new(seq, hash);
                let response = self
                    .handle_non_read(
                        &mut out.sol,
                        database,
                        request.header.function,
                        last,
                        selection,
                        objects,
                    )
                    .await;
                match response {
                    Some(response) => {
                        self.respond(io, &mut out.writer, info.addr, last, response, database)
                            .await
                    }
                    None => {
                        self.respond_with_nothing(&mut out.sol, last);
                        Ok(None)
                    }
                }
            }
            FragmentType::RepeatNonRead(_) => {
                // IEEE 1815-2012 Table 6-1: send the same response, byte for byte, which waits for
                // its confirm again if it asked for one
                self.repeat_solicited(io, info.addr, out).await?;
                Ok(out.sol.header().and_then(Self::awaited_confirm))
            }
            FragmentType::Broadcast(mode) => {
                let action = self.process_broadcast(database, mode, request).await;
                return Ok(Accepted {
                    confirm: None,
                    broadcast: Some(action),
                });
            }
        }?;

        Ok(Accepted {
            confirm,
            broadcast: None,
        })
    }

    /// Answer a request with an unknown function code with IIN2.0. It ends the selection and
    /// discards a deferred READ, but isn't a valid request, so the last valid request and its
    /// response stay (TB2016-001).
    pub(super) async fn answer_unknown_function<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        respond_to: FragmentAddr,
        seq: Sequence,
        database: &DatabaseHandle,
    ) -> Result<(), LinkError> {
        self.state.select = None;
        self.state.deferred_read.clear();
        let iin = Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT;
        self.write_transient_solicited(
            io,
            writer,
            respond_to,
            ResponseHeader::solicited(seq, iin),
            database,
        )
        .await
    }

    /// The sequence number to answer a request that couldn't be accepted with, if it gets an answer
    pub(super) fn error_response_seq(err: TransportRequestError) -> Option<Sequence> {
        match err {
            TransportRequestError::HeaderParseError(err) => match err {
                HeaderParseError::UnknownFunction(seq, _) => Some(seq),
                HeaderParseError::InsufficientBytes => None,
            },
            TransportRequestError::RequestValidationError(_, err) => {
                // IEEE 1815-2012 clause 6 discards a fragment that isn't a valid request
                tracing::warn!("ignoring invalid request: {err}");
                None
            }
        }
    }

    pub(super) fn format_first_read_response<'a>(
        sol: &'a mut ResponseSlot,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> Prepared<'a> {
        let iin2 = database.select(&object_headers);
        Self::format_read_response(sol, database, true, seq, iin2)
    }

    pub(super) fn format_read_response<'a>(
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
                parameter_error(seq)
            }
        };
        staged.with_header(header)
    }
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

impl<T: Transport> OutstationTask<T> {
    pub(super) async fn handle_one_request_from_idle(
        &mut self,
        io: &mut PhysLayer,
    ) -> Result<SolicitedWait, RunError> {
        let guard = self.session.pop_request(&mut self.reader);
        let confirm = match self.session.receive(&guard) {
            Intake::Nothing => None,
            Intake::UnknownFunction(from, seq) => {
                self.session
                    .answer_unknown_function(
                        io,
                        &mut self.responder.writer,
                        from,
                        seq,
                        &self.database,
                    )
                    .await?;
                None
            }
            Intake::Confirm(_, header) => {
                self.session.info.process_request_from_idle(header);
                let kind = if header.control.uns {
                    "unsolicited"
                } else {
                    "solicited"
                };
                tracing::warn!(
                    "ignoring {kind} CONFIRM from idle state with seq: {}",
                    header.control.seq.value()
                );
                None
            }
            Intake::Request(received) => {
                let selection = self.session.take_selection(&received.fragment);
                self.session
                    .info
                    .process_request_from_idle(received.request.header);
                self.session
                    .accept(
                        io,
                        &mut self.responder,
                        &mut self.database,
                        received,
                        selection,
                        Reads::Answer,
                    )
                    .await?
                    .confirm
            }
        };
        drop(guard);

        let Some(series) = confirm else {
            return Ok(SolicitedWait::Done);
        };

        // enter the solicited confirm wait state
        self.sol_confirm_wait(io, series).await
    }

    pub(super) async fn handle_deferred_read(
        &mut self,
        io: &mut PhysLayer,
    ) -> Result<SolicitedWait, RunError> {
        let Some(x) = self.session.state.deferred_read.select(&self.database) else {
            return Ok(SolicitedWait::Done);
        };

        tracing::info!("handling deferred READ request");
        let response = OutstationSession::format_read_response(
            &mut self.responder.sol,
            &mut self.database,
            true,
            x.seq,
            x.iin2,
        );
        let confirm = self
            .session
            .respond(
                io,
                &mut self.responder.writer,
                x.info.addr,
                LastValidRequest::new(x.seq, x.hash),
                response,
                &self.database,
            )
            .await?;

        let Some(series) = confirm else {
            return Ok(SolicitedWait::Done);
        };

        // enter the solicited confirm wait state
        self.sol_confirm_wait(io, series).await
    }
}
