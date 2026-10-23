// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::format::write::HeaderWriter;
use crate::master::error::TaskError;
use crate::master::promise::Promise;
use crate::master::request::ReadRequest;
use crate::master::tasks::{AppTask, ReadTask, Task};
use crate::master::ReadHandler;

pub(crate) struct SingleReadTask {
    request: ReadRequest,
    pub(crate) custom_handler: Option<Box<dyn ReadHandler>>,
    promise: Promise<Result<(), TaskError>>,
}

impl From<SingleReadTask> for Task {
    fn from(value: SingleReadTask) -> Self {
        Task::App(AppTask::Read(ReadTask::SingleRead(value)))
    }
}

impl SingleReadTask {
    pub(crate) fn new(request: ReadRequest, promise: Promise<Result<(), TaskError>>) -> Self {
        Self {
            request,
            custom_handler: None,
            promise,
        }
    }

    pub(crate) fn new_with_custom_handler(
        request: ReadRequest,
        custom_handler: Box<dyn ReadHandler>,
        promise: Promise<Result<(), TaskError>>,
    ) -> Self {
        Self {
            request,
            custom_handler: Some(custom_handler),
            promise,
        }
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        self.request.format(writer)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err))
    }

    pub(crate) fn on_complete(self) {
        self.promise.complete(Ok(()))
    }
}
