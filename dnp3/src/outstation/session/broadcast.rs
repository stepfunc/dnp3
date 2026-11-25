// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::context::SessionContext;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{FunctionCode, ObjectParseError, RequestHeader};
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::traits::{BroadcastAction, FreezeType};
use crate::util::clock::MonotonicClock;

impl<C: MonotonicClock> SessionContext<C> {
    pub(super) async fn process_broadcast(
        &mut self,
        mode: BroadcastConfirmMode,
        header: RequestHeader,
        objects: Result<HeaderCollection<'_>, ObjectParseError>,
    ) -> BroadcastAction {
        self.state.broadcast.received(mode);
        let action = self.process_broadcast_get_action(header, objects).await;
        self.info.broadcast_received(header.function, action);
        action
    }

    async fn process_broadcast_get_action(
        &mut self,
        header: RequestHeader,
        objects: Result<HeaderCollection<'_>, ObjectParseError>,
    ) -> BroadcastAction {
        if self.config.broadcast.is_disabled() {
            tracing::warn!(
                "ignoring broadcast request (broadcast support disabled): {:?}",
                header.function
            );
            return BroadcastAction::IgnoredByConfiguration;
        }

        let objects = match objects {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!(
                    "ignoring broadcast message with bad object headers: {}",
                    err
                );
                return BroadcastAction::BadObjectHeaders;
            }
        };

        let seq = header.control.seq;

        match header.function {
            FunctionCode::Write => {
                self.handle_write(seq, objects).await;
                BroadcastAction::Processed
            }
            FunctionCode::DirectOperateNoResponse => {
                self.handle_direct_operate_no_ack(objects).await;
                BroadcastAction::Processed
            }
            FunctionCode::ImmediateFreezeNoResponse => {
                self.handle_freeze(seq, objects, header.function, FreezeType::ImmediateFreeze);
                BroadcastAction::Processed
            }
            FunctionCode::FreezeClearNoResponse => {
                self.handle_freeze(seq, objects, header.function, FreezeType::FreezeAndClear);
                BroadcastAction::Processed
            }
            FunctionCode::FreezeAtTimeNoResponse => {
                self.handle_freeze_at_time(seq, objects, header.function);
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
                tracing::warn!("unsupported broadcast function: {:?}", header.function);
                BroadcastAction::UnsupportedFunction(header.function)
            }
        }
    }
}
