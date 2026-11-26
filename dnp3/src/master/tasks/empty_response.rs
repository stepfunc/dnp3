// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::error::TaskErrorKind;
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{Headers, TaskType};

/// Any request whose response has no object headers
pub(crate) struct EmptyResponseRequest {
    function: FunctionCode,
    headers: Headers,
}

impl EmptyResponseRequest {
    pub(crate) fn new(function: FunctionCode, headers: Headers) -> Self {
        Self { function, headers }
    }
}

/// Check that a response has no object headers
pub(crate) fn expect_empty(response: &Response<'_>) -> Result<(), TaskErrorKind> {
    if response.objects?.is_empty() {
        Ok(())
    } else {
        Err(TaskErrorKind::UnexpectedResponseHeaders)
    }
}

impl OneShotRequest for EmptyResponseRequest {
    type Output = ();
    type Error = TaskErrorKind;

    fn function(&self) -> FunctionCode {
        self.function
    }

    fn task_type(&self) -> TaskType {
        TaskType::GenericEmptyResponse(self.function)
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskErrorKind> {
        self.headers.write(writer)
    }

    fn parse(&self, response: Response<'_>) -> Result<(), TaskErrorKind> {
        expect_empty(&response)
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::EmptyResponseTask(task)
    }
}
