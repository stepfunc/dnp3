// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::context::{parameter_error, SessionContext};
use super::response::{Prepared, ResponseSlot};
use super::state::{ControlRequest, LastValidRequest, UnsolicitedState};
use crate::app::format::write::HeaderWriter;
use crate::app::gen::count::CountVariation;
use crate::app::parse::parser::{HeaderCollection, ObjectHeader};
use crate::app::parse::traits::FixedSizeVariation;
use crate::app::variations::{Group52Var1, Group52Var2};
use crate::app::{FunctionCode, Iin, Iin2, ResponseHeader, Sequence};
use crate::outstation::control::select::SelectState;
use crate::outstation::database::assign::Assignments;
use crate::outstation::database::EventClass;
use crate::outstation::target::{FreezeTarget, Selector};
use crate::outstation::traits::{FreezeInterval, FreezeType, RestartDelay};
use crate::util::clock::MonotonicClock;
use crate::util::skipped::Skipped;

impl<C: MonotonicClock> SessionContext<C> {
    pub(super) async fn handle_non_read<'a>(
        &mut self,
        sol: &'a mut ResponseSlot,
        function: FunctionCode,
        request: LastValidRequest,
        selection: Option<SelectState>,
        object_headers: HeaderCollection<'_>,
    ) -> Option<Prepared<'a>> {
        let LastValidRequest { seq, request_hash } = request;
        let extra_iin2 = get_iin2(function, object_headers);

        let mut response = match function {
            FunctionCode::Write => {
                let header = self.handle_write(seq, object_headers).await;
                sol.empty(header)
            }
            // These functions don't process objects.
            FunctionCode::DelayMeasure => {
                let g52v2 = Group52Var2 {
                    time: self.application.get_processing_delay_ms(),
                };
                single_object_response(sol, seq, g52v2)
            }
            FunctionCode::RecordCurrentTime => {
                let header = self.handle_record_current_time(seq);
                sol.empty(header)
            }
            FunctionCode::ColdRestart => {
                let delay = self.application.cold_restart();
                handle_restart(sol, seq, delay)
            }
            FunctionCode::WarmRestart => {
                let delay = self.application.warm_restart();
                handle_restart(sol, seq, delay)
            }
            // Controls
            FunctionCode::Select => {
                self.handle_controls(
                    sol,
                    ControlRequest::Select { request_hash },
                    seq,
                    object_headers,
                )
                .await
            }
            FunctionCode::Operate => {
                self.handle_controls(
                    sol,
                    ControlRequest::Operate { selection },
                    seq,
                    object_headers,
                )
                .await
            }
            FunctionCode::DirectOperate => {
                self.handle_controls(sol, ControlRequest::DirectOperate, seq, object_headers)
                    .await
            }
            FunctionCode::DirectOperateNoResponse => {
                self.handle_direct_operate_no_ack(object_headers).await;
                return None;
            }
            FunctionCode::ImmediateFreeze => {
                let header =
                    self.handle_freeze(seq, object_headers, function, FreezeType::ImmediateFreeze);
                sol.empty(header)
            }
            FunctionCode::ImmediateFreezeNoResponse => {
                self.handle_freeze(seq, object_headers, function, FreezeType::ImmediateFreeze);
                return None;
            }
            FunctionCode::FreezeClear => {
                let header =
                    self.handle_freeze(seq, object_headers, function, FreezeType::FreezeAndClear);
                sol.empty(header)
            }
            FunctionCode::FreezeClearNoResponse => {
                self.handle_freeze(seq, object_headers, function, FreezeType::FreezeAndClear);
                return None;
            }
            FunctionCode::FreezeAtTime => {
                let header = self.handle_freeze_at_time(seq, object_headers, function);
                sol.empty(header)
            }
            FunctionCode::FreezeAtTimeNoResponse => {
                self.handle_freeze_at_time(seq, object_headers, function);
                return None;
            }
            FunctionCode::EnableUnsolicited => {
                let header = self.handle_enable_or_disable_unsolicited(true, seq, object_headers);
                sol.empty(header)
            }
            FunctionCode::DisableUnsolicited => {
                let header = self.handle_enable_or_disable_unsolicited(false, seq, object_headers);
                sol.empty(header)
            }
            FunctionCode::AssignClass => {
                let header = self.handle_assign_class(seq, object_headers).await;
                sol.empty(header)
            }
            _ => {
                tracing::warn!("unsupported function code: {:?}", function);
                sol.empty(ResponseHeader::solicited(
                    seq,
                    Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT,
                ))
            }
        };

        response.header_mut().iin |= extra_iin2;
        Some(response)
    }

    pub(super) fn handle_record_current_time(&mut self, seq: Sequence) -> ResponseHeader {
        self.state.last_recorded_time = Some(self.clock.now());
        ResponseHeader::solicited(seq, Iin::default())
    }

    pub(super) fn handle_enable_or_disable_unsolicited(
        &mut self,
        enable: bool,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> ResponseHeader {
        let function = if enable {
            FunctionCode::EnableUnsolicited
        } else {
            FunctionCode::DisableUnsolicited
        };

        if self.state.unsolicited == UnsolicitedState::Disabled {
            tracing::warn!("received {function:?} request, but unsolicited support is disabled by configuration");
            return ResponseHeader::solicited(seq, Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT);
        }

        let mut iin2 = Iin2::default();
        let mut selector = Selector::new(function);

        let classes = &mut self.state.enabled_unsolicited_classes;
        for header in object_headers.iter() {
            match selector.select::<EventClass>(header) {
                Ok(EventClass::Class1) => classes.class1 = enable,
                Ok(EventClass::Class2) => classes.class2 = enable,
                Ok(EventClass::Class3) => classes.class3 = enable,
                Err(err) => iin2 |= err,
            }
        }
        selector.log();

        ResponseHeader::solicited(seq, Iin::default() | iin2)
    }

    /// Apply an ASSIGN_CLASS request to the database, then report it to the application.
    async fn handle_assign_class(
        &mut self,
        seq: Sequence,
        object_headers: HeaderCollection<'_>,
    ) -> ResponseHeader {
        if !self.application.support_assign_class() {
            tracing::warn!("received ASSIGN_CLASS request, but the application doesn't support it");
            return ResponseHeader::solicited(seq, Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT);
        }
        // An assignment set needs at least a class header.
        if object_headers.is_empty() {
            tracing::warn!("ASSIGN_CLASS request has no object headers");
            return ResponseHeader::solicited(seq, Iin::default() | Iin2::PARAMETER_ERROR);
        }
        if let Err(iin2) = self
            .db
            .with_database(|db| db.inner.assign_classes(&object_headers))
        {
            return ResponseHeader::solicited(seq, Iin::default() | iin2);
        }

        self.application.begin_assign_class();
        for assignment in Assignments::new(&object_headers).flatten() {
            self.application.assign_class(
                assignment.point_type,
                assignment.indices,
                assignment.class,
            );
        }
        self.application.end_assign_class().get().await;
        ResponseHeader::solicited(seq, Iin::default())
    }

    pub(super) fn handle_freeze(
        &mut self,
        seq: Sequence,
        object_headers: HeaderCollection,
        function: FunctionCode,
        freeze_type: FreezeType,
    ) -> ResponseHeader {
        let mut iin = Iin::default();
        let mut selector = Selector::new(function);

        for header in object_headers.iter() {
            iin |= self.handle_freeze_header(&mut selector, freeze_type, header);
        }
        selector.log();

        ResponseHeader::solicited(seq, iin)
    }

    pub(super) fn handle_freeze_at_time(
        &mut self,
        seq: Sequence,
        object_headers: HeaderCollection,
        function: FunctionCode,
    ) -> ResponseHeader {
        let mut iin = Iin::default();

        let mut timing: Option<FreezeInterval> = None;
        let mut selector = Selector::new(function);
        let mut without_timing = Skipped::new();

        for header in object_headers.iter() {
            match header {
                ObjectHeader::Count(_, _, CountVariation::Group50Var2(seq)) => match seq.single() {
                    None => iin.iin2 |= Iin2::PARAMETER_ERROR,
                    Some(x) => {
                        timing = Some(x.into());
                    }
                },
                ObjectHeader::AllObjects(_)
                | ObjectHeader::Range(..)
                | ObjectHeader::Count(..)
                | ObjectHeader::CountAndPrefix8(..)
                | ObjectHeader::CountAndPrefix16(..)
                | ObjectHeader::FreeFormat(_)
                | ObjectHeader::Read(..) => {
                    // Other variations require that there be a preceding g50v2.
                    match timing {
                        None => {
                            without_timing.add(header.variation());
                            iin.iin2 |= Iin2::PARAMETER_ERROR;
                        }
                        Some(x) => {
                            iin |= self.handle_freeze_header(
                                &mut selector,
                                FreezeType::FreezeAtTime(x),
                                header,
                            );
                        }
                    }
                }
            }
        }

        selector.log();
        if let Some((variation, count)) = without_timing.first() {
            tracing::warn!("freeze-at-time on {variation} w/o preceding g50v2{count}");
        }

        ResponseHeader::solicited(seq, iin)
    }

    fn handle_freeze_header(
        &mut self,
        selector: &mut Selector,
        freeze_type: FreezeType,
        header: ObjectHeader,
    ) -> Iin2 {
        match selector.select::<FreezeTarget>(header) {
            Ok(indices) => self
                .application
                .freeze_counter(indices, freeze_type, &mut self.db)
                .map_or_else(|err| err.into(), |_| Iin2::default()),
            Err(err) => err,
        }
    }
}

fn get_iin2(function: FunctionCode, object_headers: HeaderCollection) -> Iin2 {
    if function.objects_allowed() {
        return Iin2::default();
    }

    if object_headers.is_empty() {
        Iin2::default()
    } else {
        tracing::warn!("ignoring object headers in {:?} request", function);
        Iin2::PARAMETER_ERROR
    }
}

/// Respond with a single object, or with IIN2.2 if it can't be written.
fn single_object_response<V>(sol: &mut ResponseSlot, seq: Sequence, item: V) -> Prepared<'_>
where
    V: FixedSizeVariation,
{
    let (result, staged) = sol.build(|cursor| HeaderWriter::new(cursor).write_count_of_one(item));
    let header = match result {
        Ok(()) => ResponseHeader::solicited(seq, Iin::default()),
        Err(err) => {
            tracing::error!("unable to write {item} to the response buffer: {err:?}");
            parameter_error(seq)
        }
    };
    staged.with_header(header)
}

fn handle_restart(
    sol: &mut ResponseSlot,
    seq: Sequence,
    delay: Option<RestartDelay>,
) -> Prepared<'_> {
    let delay = match delay {
        None => {
            return sol.empty(ResponseHeader::solicited(
                seq,
                Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT,
            ));
        }
        Some(x) => x,
    };

    // Respond with the delay.
    match delay {
        RestartDelay::Seconds(value) => {
            single_object_response(sol, seq, Group52Var1 { time: value })
        }
        RestartDelay::Milliseconds(value) => {
            single_object_response(sol, seq, Group52Var2 { time: value })
        }
    }
}
