// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

impl OutstationHandler {
    pub(super) async fn process_broadcast(
        &mut self,
        database: &mut DatabaseHandle,
        mode: BroadcastConfirmMode,
        request: Request<'_>,
    ) -> BroadcastAction {
        self.state.broadcast.received(mode);
        let action = self.process_broadcast_get_action(database, request).await;
        self.info
            .broadcast_received(request.header.function, action);
        action
    }

    pub(super) async fn process_broadcast_get_action(
        &mut self,
        database: &mut DatabaseHandle,
        request: Request<'_>,
    ) -> BroadcastAction {
        if self.config.broadcast.is_disabled() {
            tracing::warn!(
                "ignoring broadcast request (broadcast support disabled): {:?}",
                request.header.function
            );
            return BroadcastAction::IgnoredByConfiguration;
        }

        let objects = match request.objects {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!(
                    "ignoring broadcast message with bad object headers: {}",
                    err
                );
                return BroadcastAction::BadObjectHeaders;
            }
        };

        let seq = request.header.control.seq;

        match request.header.function {
            FunctionCode::Write => {
                self.handle_write(seq, objects, database).await;
                BroadcastAction::Processed
            }
            FunctionCode::DirectOperateNoResponse => {
                self.handle_direct_operate_no_ack(database, objects).await;
                BroadcastAction::Processed
            }
            FunctionCode::ImmediateFreezeNoResponse => {
                self.handle_freeze(
                    database,
                    seq,
                    objects,
                    request.header.function,
                    FreezeType::ImmediateFreeze,
                );
                BroadcastAction::Processed
            }
            FunctionCode::FreezeClearNoResponse => {
                self.handle_freeze(
                    database,
                    seq,
                    objects,
                    request.header.function,
                    FreezeType::FreezeAndClear,
                );
                BroadcastAction::Processed
            }
            FunctionCode::FreezeAtTimeNoResponse => {
                self.handle_freeze_at_time(database, seq, objects, request.header.function);
                BroadcastAction::Processed
            }
            FunctionCode::RecordCurrentTime => {
                self.handle_record_current_time(seq);
                BroadcastAction::Processed
            }
            FunctionCode::DisableUnsolicited => {
                self.handle_enable_or_disable_unsolicited(false, seq, objects);
                BroadcastAction::Processed
            }
            FunctionCode::EnableUnsolicited => {
                self.handle_enable_or_disable_unsolicited(true, seq, objects);
                BroadcastAction::Processed
            }
            _ => {
                tracing::warn!(
                    "unsupported broadcast function: {:?}",
                    request.header.function
                );
                BroadcastAction::UnsupportedFunction(request.header.function)
            }
        }
    }
}
