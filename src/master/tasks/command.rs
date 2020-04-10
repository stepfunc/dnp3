use crate::app::format::write::start_request;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::{Control, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::task::{ResponseError, ResponseResult};
use crate::master::types::CommandHeader;
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

    pub(crate) fn handle(
        &mut self,
        _response: ResponseHeader,
        _headers: HeaderCollection,
    ) -> Result<ResponseResult, ResponseError> {
        Ok(ResponseResult::Success)
    }
}
