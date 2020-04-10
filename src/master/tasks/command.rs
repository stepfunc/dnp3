use crate::app::gen::enums::FunctionCode;
use crate::app::header::{Control, RequestHeader, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::task::{ResponseError, ResponseResult};
use crate::master::types::{CommandHeader, CommandRequest};
use crate::util::cursor::{WriteCursor, WriteError};

pub(crate) struct CommandTask {
    pub(crate) request: CommandRequest,
}

impl CommandTask {
    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        RequestHeader::new(Control::request(seq), FunctionCode::DirectOperate).write(cursor)?;

        for header in self.request.headers.iter() {
            match header {
                CommandHeader::G12V1PrefixedU8(_items) => {}
                _ => {}
            }
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
