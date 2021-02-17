use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::outstation::traits::{BroadcastAction, OperateType, RestartDelay};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::app::FunctionCode;

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
}

#[derive(Clone)]
pub(crate) struct EventHandle {
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl EventHandle {
    pub(crate) fn new() -> Self {
        EventHandle {
            events: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub(crate) fn push(&self, event: Event) {
        self.events.lock().unwrap().push_back(event);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    pub(crate) fn pop(&self) -> Option<Event> {
        self.events.lock().unwrap().pop_front()
    }
}
