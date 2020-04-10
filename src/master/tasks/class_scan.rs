use crate::app::format::write::start_request;
use crate::app::gen::enums::FunctionCode;
use crate::app::gen::variations::variation::Variation;
use crate::app::header::{Control, ResponseHeader};
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
        let mut writer = start_request(Control::request(seq), FunctionCode::Read, cursor)?;
        if self.scan.class1 {
            writer.write_all_objects_header(Variation::Group60Var2)?;
        }
        if self.scan.class2 {
            writer.write_all_objects_header(Variation::Group60Var3)?;
        }
        if self.scan.class3 {
            writer.write_all_objects_header(Variation::Group60Var4)?;
        }
        if self.scan.class0 {
            writer.write_all_objects_header(Variation::Group60Var1)?;
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
