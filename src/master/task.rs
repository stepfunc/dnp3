use crate::app::format::write;
use crate::app::gen::enums::FunctionCode;
use crate::app::gen::variations::gv::Variation;
use crate::app::header::{Control, RequestHeader};
use crate::app::parse::parser::{ObjectParseError, Response};
use crate::app::sequence::Sequence;
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
    Complete,
    /// another response is needed
    Continue,
    ///// run a new task - e.g. select then operate
    //Transition(Box<dyn MasterTask>),
}

pub enum TaskDetails {
    ClassScan(ClassScan),
}

impl TaskDetails {
    pub fn function(&self) -> FunctionCode {
        match self {
            TaskDetails::ClassScan(_) => FunctionCode::Read,
        }
    }

    pub fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        match self {
            TaskDetails::ClassScan(params) => {
                RequestHeader::new(Control::request(seq), self.function()).write(cursor)?;
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

    pub fn handle(&self, response: Response) -> Result<ResponseResult, ResponseError> {
        match self {
            TaskDetails::ClassScan(_) => {
                log::info!(
                    "fir: {} fin: {} seq: {}",
                    response.header.control.fir,
                    response.header.control.fin,
                    response.header.control.seq.value()
                );

                for _ in response.parse_objects()? {
                    log::info!("got a header");
                }

                if response.header.control.fin {
                    Ok(ResponseResult::Complete)
                } else {
                    Ok(ResponseResult::Continue)
                }
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
