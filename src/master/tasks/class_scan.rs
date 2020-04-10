use crate::app::format::write;
use crate::app::gen::enums::FunctionCode;
use crate::app::gen::variations::variation::Variation;
use crate::app::header::{Control, RequestHeader, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::handlers::ResponseHandler;
use crate::master::task::{ResponseError, ResponseResult};
use crate::master::types::ClassScan;
use crate::util::cursor::{WriteCursor, WriteError};

pub(crate) struct ClassScanTask {
    pub(crate) scan: ClassScan,
    pub(crate) handler: Box<dyn ResponseHandler>,
}

impl ClassScanTask {
    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        RequestHeader::new(Control::request(seq), FunctionCode::Read).write(cursor)?;
        if self.scan.class1 {
            write::write_all_objects(Variation::Group60Var2, cursor)?;
        }
        if self.scan.class2 {
            write::write_all_objects(Variation::Group60Var3, cursor)?;
        }
        if self.scan.class3 {
            write::write_all_objects(Variation::Group60Var4, cursor)?;
        }
        if self.scan.class0 {
            write::write_all_objects(Variation::Group60Var1, cursor)?;
        }
        Ok(())
    }

    pub(crate) fn handle(
        &mut self,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> Result<ResponseResult, ResponseError> {
        // TODO - provide the proper addressing
        self.handler.handle(1024, response, headers);
        Ok(ResponseResult::Success)
    }
}
