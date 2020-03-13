use crate::error::{Error, LogicError};
use crate::link::parser::{Frame, Parser};
use crate::util::cursor::{ReadCursor, ReadError};
use crate::util::slice_ext::*;
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct Reader {
    parser: Parser,
    begin: usize,
    end: usize,
    buffer: [u8; super::constant::MAX_LINK_FRAME_LENGTH],
}

impl Reader {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            begin: 0,
            end: 0,
            buffer: [0; super::constant::MAX_LINK_FRAME_LENGTH],
        }
    }

    /*
        pub async fn read<'a>(parser: &'a mut Parser) -> Result<Frame<'a>, Error> {

            let mut cursor = ReadCursor::new(&[]);
            let cursor_ref = &mut cursor;

            loop {

                //let unread = &mut self.buffer[self.begin..self.end];

                //let start_length = cursor.len();
                match parser.parse(cursor_ref)? {
                    Some(frame) => return Ok(frame),
                    None => {},

                }

            }

            Err(Error::BadLogic(LogicError::BadRead))
        }
    */
}
