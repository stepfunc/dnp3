use crate::app::format::write;
use crate::app::gen::enums::FunctionCode;
use crate::app::gen::variations::gv::Variation;
use crate::app::header::{Control, RequestHeader};
use crate::app::parse::parser::{ObjectParseError, Response};
use crate::app::sequence::Sequence;
use crate::master::handlers::ResponseHandler;
use crate::master::types::ClassScan;
use crate::util::cursor::{WriteCursor, WriteError};

#[derive(Copy, Clone, Debug)]
pub enum ResponseError {
    BadObjects(ObjectParseError),
}

impl std::convert::From<ObjectParseError> for ResponseError {
    fn from(err: ObjectParseError) -> Self {
        ResponseError::BadObjects(err)
    }
}

pub enum ResponseResult {
    /// the response completed the task
    Success,
    ///// run a new task - e.g. select then operate
    //Transition(MasterTask),
}

pub enum TaskDetails {
    ClassScan(ClassScan, Box<dyn ResponseHandler>),
}

impl TaskDetails {
    pub fn is_read_request(&self) -> bool {
        match self {
            TaskDetails::ClassScan(_, _) => true,
        }
    }

    pub fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        match self {
            TaskDetails::ClassScan(params, _) => {
                RequestHeader::new(Control::request(seq), FunctionCode::Read).write(cursor)?;
                if params.class1 {
                    write::write_all_objects(Variation::Group60Var2, cursor)?;
                }
                if params.class2 {
                    write::write_all_objects(Variation::Group60Var3, cursor)?;
                }
                if params.class3 {
                    write::write_all_objects(Variation::Group60Var4, cursor)?;
                }
                if params.class0 {
                    write::write_all_objects(Variation::Group60Var1, cursor)?;
                }
                Ok(())
            }
        }
    }

    pub fn handle(&mut self, response: Response) -> Result<ResponseResult, ResponseError> {
        match self {
            TaskDetails::ClassScan(_, handler) => {
                handler.handle(1024, response.header, response.parse_objects()?);
                Ok(ResponseResult::Success)
            }
        }
    }
}

pub struct MasterTask {
    pub destination: u16,
    pub details: TaskDetails,
}

impl MasterTask {
    pub fn new(destination: u16, details: TaskDetails) -> Self {
        Self {
            destination,
            details,
        }
    }
}
