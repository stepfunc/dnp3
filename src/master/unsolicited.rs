use crate::app::parse::parser::{ParseLogLevel, Response};
use crate::link::error::LinkError;
use crate::master::handlers::ResponseHandler;

use crate::app::sequence::Sequence;
use crate::transport::WriterType;
use crate::util::cursor::WriteCursor;
use tokio::prelude::{AsyncRead, AsyncWrite};

pub(crate) struct UnsolicitedHandler {
    // TODO - track the sequence number, etc
    handler: Box<dyn ResponseHandler>,
}

impl UnsolicitedHandler {
    pub(crate) fn new(handler: Box<dyn ResponseHandler>) -> Self {
        Self { handler }
    }

    pub(crate) async fn handle<T>(
        &mut self,
        level: ParseLogLevel,
        source: u16,
        response: &Response<'_>,
        io: &mut T,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if let Ok(objects) = response.objects {
            self.handler.handle(source, response.header, objects);
        }

        if response.header.control.con {
            Self::confirm(level, io, source, response.header.control.seq, writer).await?;
        }

        Ok(())
    }

    async fn confirm<T>(
        level: ParseLogLevel,
        io: &mut T,
        destination: u16,
        seq: Sequence,
        writer: &mut WriterType,
    ) -> Result<(), LinkError>
    where
        T: AsyncWrite + Unpin,
    {
        let mut buffer: [u8; 2] = [0; 2];
        let mut cursor = WriteCursor::new(&mut buffer);
        crate::app::format::write::confirm_unsolicited(seq, &mut cursor)?;

        writer
            .write(level, io, destination, cursor.written())
            .await?;
        Ok(())
    }
}
