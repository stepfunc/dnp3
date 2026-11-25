// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::context::{parameter_error, SessionContext};
use super::response::{Prepared, ResponseSlot, Staged};
use super::state::ControlRequest;
use crate::app::control::CommandStatus;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{Iin, Iin2, ResponseHeader, Sequence};
use crate::outstation::control::collection::{
    ControlCollection, ControlLimit, ControlMode, ControlTransaction,
};
use crate::outstation::control::select::SelectState;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::OperateType;
use crate::util::clock::MonotonicClock;

impl<C: MonotonicClock> SessionContext<C> {
    pub(super) async fn handle_controls<'a>(
        &mut self,
        sol: &'a mut ResponseSlot,
        request: ControlRequest,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection<'_>,
    ) -> Prepared<'a> {
        let Some(controls) = controls(object_headers) else {
            return sol.empty(ResponseHeader::solicited(
                seq,
                Iin::default() | Iin2::OBJECT_UNKNOWN,
            ));
        };

        if !control_response_fits(sol, &controls) {
            tracing::warn!(
                "ignoring control request whose response doesn't fit in the response buffer"
            );
            return sol.empty(parameter_error(seq));
        }

        let (result, staged) = match request {
            ControlRequest::Select { request_hash } => {
                let (result, staged) = self
                    .execute_controls(sol, database, ControlMode::Select, &controls)
                    .await;
                if let Ok(CommandStatus::Success) = result {
                    // Sample after the async control callback so the select timeout starts when
                    // the selection completes, not when the request arrived.
                    self.state.select = Some(SelectState::new(
                        seq,
                        request_hash,
                        self.clock.now(),
                        controls.hash(),
                    ));
                }
                (result, staged)
            }
            ControlRequest::Operate { selection } => {
                // The OPERATE has already ended the selection, so it is used at most once.
                let matched = selection.ok_or(CommandStatus::NoSelect).and_then(|s| {
                    s.match_operate(
                        self.clock.now(),
                        self.config.select_timeout,
                        seq,
                        controls.hash(),
                    )
                });
                match matched {
                    Err(status) => sol.build(|cursor| {
                        controls.respond_with_status(cursor, status).map(|_| status)
                    }),
                    Ok(()) => {
                        self.execute_controls(
                            sol,
                            database,
                            ControlMode::Operate(OperateType::SelectBeforeOperate),
                            &controls,
                        )
                        .await
                    }
                }
            }
            ControlRequest::DirectOperate => {
                self.execute_controls(
                    sol,
                    database,
                    ControlMode::Operate(OperateType::DirectOperate),
                    &controls,
                )
                .await
            }
        };

        control_response(seq, result, staged)
    }

    pub(super) async fn handle_direct_operate_no_ack(
        &mut self,
        database: &mut DatabaseHandle,
        object_headers: HeaderCollection<'_>,
    ) {
        let Some(controls) = controls(object_headers) else {
            return;
        };
        let max_controls_per_request = self.config.max_controls_per_request;
        ControlTransaction::execute(self.control_handler.as_mut(), database, |tx, db| {
            controls.operate_no_ack(tx, db, ControlLimit::new(max_controls_per_request))
        })
        .await;
    }

    /// Execute the controls and build the response that echoes them, returning their overall
    /// status.
    async fn execute_controls<'a>(
        &mut self,
        sol: &'a mut ResponseSlot,
        database: &mut DatabaseHandle,
        mode: ControlMode,
        controls: &ControlCollection<'_>,
    ) -> (Result<CommandStatus, scursor::WriteError>, Staged<'a>) {
        let max_controls_per_request = self.config.max_controls_per_request;
        ControlTransaction::execute(self.control_handler.as_mut(), database, move |tx, db| {
            sol.build(|cursor| {
                controls.execute_with_response(
                    cursor,
                    mode,
                    tx,
                    db,
                    ControlLimit::new(max_controls_per_request),
                )
            })
        })
        .await
    }
}

/// The controls in a request, or `None` if it has an object that the control functions don't
/// support. A control object with an unsupported qualifier fails to parse, so it's one of those.
fn controls(object_headers: HeaderCollection<'_>) -> Option<ControlCollection<'_>> {
    match ControlCollection::from(object_headers) {
        Ok(controls) => Some(controls),
        Err(err) => {
            tracing::warn!(
                "ignoring control request containing non-control object header {} - {}",
                err.variation,
                err.qualifier
            );
            None
        }
    }
}

/// Echo the controls into an unpublished response, without executing them, to check that the
/// real response fits. The echo is the same size whatever the status.
fn control_response_fits(sol: &mut ResponseSlot, controls: &ControlCollection) -> bool {
    let (result, _) =
        sol.build(|cursor| controls.respond_with_status(cursor, CommandStatus::Success));
    result.is_ok()
}

/// `result` is the overall status of the controls built into the response, or the error writing
/// them.
fn control_response(
    seq: Sequence,
    result: Result<CommandStatus, scursor::WriteError>,
    staged: Staged<'_>,
) -> Prepared<'_> {
    let header = match result {
        Ok(status) => {
            let mut iin = Iin::default();
            if status == CommandStatus::NotSupported {
                iin |= Iin2::PARAMETER_ERROR;
            }
            ResponseHeader::solicited(seq, iin)
        }
        Err(_) => {
            tracing::warn!("control response doesn't fit in the response buffer");
            parameter_error(seq)
        }
    };
    staged.with_header(header)
}
