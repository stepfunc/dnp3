// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::outstation::traits::{BroadcastAction, OperateType, RestartDelay};
use crate::outstation::{BufferState, FreezeIndices, FreezeType};

use crate::app::attr::AttrSet;
use crate::app::{ControlField, FunctionCode, RequestHeader, Sequence, Timestamp};

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Control {
    G12V1(Group12Var1, u16),
    G41V1(Group41Var1, u16),
    G41V2(Group41Var2, u16),
    G41V3(Group41Var3, u16),
    G41V4(Group41Var4, u16),
}

/// the g41v2 control with value 513 at index 7 that tests in several files send
pub(crate) const G41V2_INDEX_7: Control = Control::G41V2(Group41Var2::new(513), 7);

/// the events of a SELECT of [`G41V2_INDEX_7`]
pub(crate) fn selected() -> [Event; 3] {
    [
        Event::BeginControls,
        Event::Select(G41V2_INDEX_7),
        Event::EndControls,
    ]
}

/// the information callback for a request with one fragment, from idle
pub(crate) fn from_idle(function: FunctionCode, uns: bool, seq: u8) -> Event {
    Event::RequestFromIdle(RequestHeader {
        control: ControlField {
            fir: true,
            fin: true,
            con: false,
            uns,
            seq: Sequence::new(seq),
        },
        function,
    })
}

/// the events of an operation of [`G41V2_INDEX_7`]
pub(crate) fn operated(operate_type: OperateType) -> [Event; 3] {
    [
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, operate_type),
        Event::EndControls,
    ]
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
    /// the application was asked to validate the WRITE of the attribute (set, variation)
    WriteDeviceAttr(AttrSet, u8),
    BeginConfirm,
    Cleared(u64),
    EndConfirm(BufferState),
    RequestFromIdle(RequestHeader),
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
