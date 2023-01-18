use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::outstation::traits::{BroadcastAction, OperateType, RestartDelay};
use crate::outstation::{BufferState, FreezeIndices, FreezeType};

use crate::app::{FunctionCode, Timestamp};

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Control {
    G12V1(Group12Var1, u16),
    G41V1(Group41Var1, u16),
    G41V2(Group41Var2, u16),
    G41V3(Group41Var3, u16),
    G41V4(Group41Var4, u16),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Event {
    BeginControls,
    Select(Control),
    Operate(Control, OperateType),
    Freeze(FreezeIndices, FreezeType),
    EndControls,
    BroadcastReceived(FunctionCode, BroadcastAction),
    EnterSolicitedConfirmWait(u8),
    EnterUnsolicitedConfirmWait(u8),
    SolicitedConfirmTimeout(u8),
    UnsolicitedConfirmTimeout(u8, bool),
    UnsolicitedConfirmReceived(u8),
    SolicitedConfirmReceived(u8),
    SolicitedConfirmWaitNewRequest,
    UnexpectedConfirm(bool, u8),
    WrongSolicitedConfirmSeq(u8, u8),
    ColdRestart(Option<RestartDelay>),
    WarmRestart(Option<RestartDelay>),
    ClearRestartIIN,
    WriteAbsoluteTime(Timestamp),
    BeginWriteDeadBands,
    WriteDeadBand(u16, f64),
    EndWriteDeadBands,
    BeginConfirm,
    Cleared(u64),
    EndConfirm(BufferState),
}

#[derive(Clone)]
pub(crate) struct EventSender {
    tx: tokio::sync::mpsc::UnboundedSender<Event>,
}

pub(crate) struct EventReceiver {
    rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
}

pub(crate) fn event_handlers() -> (EventSender, EventReceiver) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (EventSender { tx }, EventReceiver { rx })
}

impl EventReceiver {
    pub(crate) async fn next(&mut self) -> Event {
        self.rx.recv().await.unwrap()
    }

    pub(crate) fn poll(&mut self) -> Option<Event> {
        self.rx.try_recv().ok()
    }
}

impl EventSender {
    pub(crate) fn send(&self, event: Event) {
        self.tx.send(event).unwrap()
    }
}
