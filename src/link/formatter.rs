use crate::link::header::{ControlField, Address};
use crate::util::cursor::{WriteCursor, WriteError};
use crate::link::constant;
use crate::error::LogicError;
use crate::link::crc::{calc_crc_with_0564, calc_crc};
use crate::util::slice_ext::SliceExtNoPanic;
use crate::link::function::Function;


impl std::convert::From<WriteError> for LogicError {
    fn from(_: WriteError) -> Self {
        LogicError::BadWrite
    }
}

#[derive(Copy, Clone)]
pub struct Payload<'a> {
    transport: u8,
    app_data: &'a [u8],
}

impl<'a> Payload<'a> {
    pub fn new(transport: u8, app_data: &'a [u8]) -> Self {
        Self { transport, app_data }
    }
}

struct LinkFormatter {
    master: bool
}

impl LinkFormatter {

    pub fn new(master: bool) -> Self {
        Self { master }
    }

    pub fn format_ack(&self, address: Address, cursor: &mut WriteCursor) -> Result<(), LogicError> {
        Self::format(
            ControlField::new(self.master, Function::SecAck),
            address,
            None,
            cursor
        )
    }

    pub fn format_unconfirmed_user_data(&self, address: Address, payload: Payload, cursor: &mut WriteCursor) -> Result<(), LogicError> {
        Self::format(
            ControlField::new(self.master, Function::PriUnconfirmedUserData),
            address,
            Some(payload),
            cursor
        )
    }

    fn format(control: ControlField, address: Address, payload: Option<Payload>, cursor: &mut WriteCursor) -> Result<(), LogicError> {

        fn format_payload(payload: Payload, cursor: &mut WriteCursor) -> Result<(), LogicError> {
            // the first block contains the transport header
            let (first, remainder) = payload.app_data.np_split_at_no_error(constant::MAX_BLOCK_SIZE - 1);

            // write the first block
            let begin_first_block = cursor.position();
            cursor.write_u8(payload.transport)?;
            cursor.write(first)?;
            cursor.write_u16_le(calc_crc(cursor.written_since(begin_first_block)?))?;

            // write the remaining blocks
            for block in remainder.chunks(constant::MAX_BLOCK_SIZE) {
                let start_block = cursor.position();
                cursor.write(block)?;
                cursor.write_u16_le(calc_crc(cursor.written_since(start_block)?))?;
            }

            Ok(())
        }

        let length: u8 = match payload {
            Some(payload) => {
                if payload.app_data.len() > constant::MAX_APP_BYTES_PER_FRAME {
                    return Err(LogicError::BadSize);
                }
                payload.app_data.len() as u8 + constant::MIN_HEADER_LENGTH_VALUE + 1
            },
            None => constant::MIN_HEADER_LENGTH_VALUE
        };

        cursor.write_u8(constant::START1)?;
        cursor.write_u8(constant::START2)?;

        let header_start = cursor.position();

        cursor.write_u8(length)?;
        cursor.write_u8(control.to_u8())?;
        cursor.write_u16_le(address.destination)?;
        cursor.write_u16_le(address.source)?;
        cursor.write_u16_le(calc_crc_with_0564(cursor.written_since(header_start)?))?;

        match payload {
            Some(payload) => format_payload(payload, cursor),
            None => Ok(()),
        }
    }
}



#[cfg(test)]
mod test {

    use super::*;
    use super::super::test_data::*;

    type Buffer = [u8; constant::MAX_LINK_FRAME_LENGTH];

    #[test]
    fn can_format_ack() {
        let mut buffer : Buffer = [0; constant::MAX_LINK_FRAME_LENGTH];
        let mut cursor = WriteCursor::new(&mut buffer);
        let start = cursor.position();
        let formatter = LinkFormatter::new(false);
        formatter.format_ack(ACK_FRAME.header.address, &mut cursor).unwrap();
        assert_eq!(cursor.written_since(start).unwrap(), ACK_BYTES);
    }

    #[test]
    fn can_format_unconfirmed_user_data() {
        let mut buffer : Buffer = [0; constant::MAX_LINK_FRAME_LENGTH];
        let mut cursor = WriteCursor::new(&mut buffer);
        let start = cursor.position();
        let formatter = LinkFormatter::new(true);
        formatter.format_unconfirmed_user_data(UNCONFIRMED_USER_DATA_FRAME.header.address, Payload::new(0xC0, &UNCONFIRMED_USER_DATA_APP_BYTES), &mut cursor).unwrap();
        assert_eq!(cursor.written_since(start).unwrap(), UNCONFIRMED_USER_DATA_BYTES);
    }
}

