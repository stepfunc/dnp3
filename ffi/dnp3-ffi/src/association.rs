use std::time::Duration;

use dnp3::link::LinkStatusResult;
use dnp3::master::PollHandle;
use dnp3::master::*;

use crate::command::Command;
use crate::ffi;
use crate::request::Request;

pub struct Association {
    pub(crate) runtime: crate::runtime::RuntimeHandle,
    pub(crate) handle: AssociationHandle,
}

pub unsafe fn association_destroy(association: *mut Association) {
    if !association.is_null() {
        let association = Box::from_raw(association);
        if let Some(rt) = association.runtime.get() {
            rt.spawn(association.handle.remove());
        }
    }
}

pub struct Poll {
    runtime: crate::runtime::RuntimeHandle,
    handle: PollHandle,
}

impl Poll {
    fn new(runtime: crate::runtime::RuntimeHandle, handle: PollHandle) -> Self {
        Self { runtime, handle }
    }
}

pub unsafe fn poll_demand(poll: *mut Poll) {
    if let Some(poll) = poll.as_mut() {
        poll.runtime.unwrap().spawn(poll.handle.demand());
    }
}

pub unsafe fn poll_destroy(poll: *mut Poll) {
    if !poll.is_null() {
        let poll = Box::from_raw(poll);
        if let Some(rt) = poll.runtime.get() {
            rt.spawn(poll.handle.remove());
        }
    }
}

pub unsafe fn association_add_poll(
    association: *mut Association,
    request: *mut Request,
    period: Duration,
) -> *mut Poll {
    let association = match association.as_mut() {
        Some(association) => association,
        None => return std::ptr::null_mut(),
    };

    let request = match request.as_ref() {
        Some(request) => request,
        None => return std::ptr::null_mut(),
    };

    if tokio::runtime::Handle::try_current().is_err() {
        if let Ok(handle) = association
            .runtime
            .unwrap()
            .block_on(association.handle.add_poll(request.build(), period))
        {
            let poll = Box::new(Poll::new(association.runtime.clone(), handle));
            Box::into_raw(poll)
        } else {
            tracing::warn!("Poll creation failure");
            std::ptr::null_mut()
        }
    } else {
        tracing::warn!("Tried calling 'association_add_poll' from within a tokio thread");
        std::ptr::null_mut()
    }
}

pub unsafe fn association_operate(
    association: *mut Association,
    mode: ffi::CommandMode,
    command: *const Command,
    callback: ffi::CommandTaskCallback,
) {
    let association = match association.as_mut() {
        Some(association) => association,
        None => {
            callback.on_complete(ffi::CommandResult::TaskError);
            return;
        }
    };

    let command = match command.as_ref() {
        Some(command) => command,
        None => {
            callback.on_complete(ffi::CommandResult::TaskError);
            return;
        }
    };

    let mode = match mode {
        ffi::CommandMode::DirectOperate => CommandMode::DirectOperate,
        ffi::CommandMode::SelectBeforeOperate => CommandMode::SelectBeforeOperate,
    };

    let handle = &mut association.handle;
    let cmd = command.clone();
    association.runtime.unwrap().spawn(async move {
        let result = match handle.operate(mode, cmd.build()).await {
            Ok(_) => ffi::CommandResult::Success,
            Err(CommandError::Task(_)) => ffi::CommandResult::TaskError,
            Err(CommandError::Response(err)) => match err {
                CommandResponseError::Request(_) => ffi::CommandResult::TaskError,
                CommandResponseError::BadStatus(_) => ffi::CommandResult::BadStatus,
                CommandResponseError::HeaderCountMismatch => {
                    ffi::CommandResult::HeaderCountMismatch
                }
                CommandResponseError::HeaderTypeMismatch => ffi::CommandResult::HeaderTypeMismatch,
                CommandResponseError::ObjectCountMismatch => {
                    ffi::CommandResult::ObjectCountMismatch
                }
                CommandResponseError::ObjectValueMismatch => {
                    ffi::CommandResult::ObjectValueMismatch
                }
            },
        };

        callback.on_complete(result);
    });
}



pub unsafe fn association_check_link_status(
    association: *mut Association,
    callback: ffi::LinkStatusCallback,
) {
    let association = match association.as_mut() {
        Some(association) => association,
        None => {
            callback.on_complete(ffi::LinkStatusResult::TaskError);
            return;
        }
    };

    let handle = &mut association.handle;
    association.runtime.unwrap().spawn(async move {
        let result = match handle.check_link_status().await {
            Ok(LinkStatusResult::Success) => ffi::LinkStatusResult::Success,
            Ok(LinkStatusResult::UnexpectedResponse) => ffi::LinkStatusResult::UnexpectedResponse,
            Err(_) => ffi::LinkStatusResult::TaskError,
        };

        callback.on_complete(result);
    });
}

