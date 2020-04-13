use crate::app::format::write::start_request;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::{Control, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::runner::TaskError;
use crate::master::task::TaskStatus;
use crate::master::types::{CommandError, CommandHeader};
use crate::util::cursor::{WriteCursor, WriteError};

pub(crate) struct CommandTask {
    headers: Vec<CommandHeader>,
}

impl CommandTask {
    pub(crate) fn new(headers: Vec<CommandHeader>) -> Self {
        Self { headers }
    }

    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        let mut writer = start_request(Control::request(seq), FunctionCode::DirectOperate, cursor)?;

        for header in self.headers.iter() {
            header.write(&mut writer)?;
        }

        Ok(())
    }

    fn compare(&self, headers: HeaderCollection) -> Result<(), CommandError> {
        let mut iter = headers.iter();

        for sent in &self.headers {
            match iter.next() {
                None => return Err(CommandError::HeaderCountMismatch),
                Some(received) => sent.compare(received.details)?,
            }
        }

        if iter.next().is_some() {
            return Err(CommandError::HeaderCountMismatch);
        }

        Ok(())
    }

    pub(crate) fn handle(
        &mut self,
        _source: u16,
        _response: ResponseHeader,
        headers: HeaderCollection,
    ) -> TaskStatus {
        let comparison = self.compare(headers);

        log::warn!("result: {:?}", comparison);

        TaskStatus::Complete
    }

    pub(crate) fn on_error(&mut self, _error: TaskError) {
        // TODO - notify some kind of handler
    }
}
