use crate::link::constant;
use crate::link::crc::{calc_crc, calc_crc_with_0564};
use crate::link::error::LogicError;
use crate::link::function::Function;
use crate::link::header::ControlField;
use crate::util::cursor::{WriteCursor, WriteError};
use crate::util::slice_ext::SliceExtNoPanic;

impl From<WriteError> for LogicError {
    fn from(_: WriteError) -> Self {
        LogicError::BadWrite
    }
}

#[derive(Copy, Clone)]
pub(crate) struct Payload<'a> {
    transport: u8,
    app_data: &'a [u8],
}

impl<'a> Payload<'a> {
    pub(crate) fn new(transport: u8, app_data: &'a [u8]) -> Self {
        Self {
            transport,
            app_data,
        }
    }
}

pub(crate) fn format_header(
    control: ControlField,
    destination: u16,
    source: u16,
    buffer: &mut [u8; super::constant::LINK_HEADER_LENGTH],
) {
    fn to_le(x: u16) -> (u8, u8) {
        let low = (x & 0xFF) as u8;
        let high = ((x >> 8) & 0xFF) as u8;
        (low, high)
    }

    buffer[0] = constant::START1;
    buffer[1] = constant::START2;
    buffer[2] = 5;
    buffer[3] = control.to_u8();
    let (d1, d2) = to_le(destination);
    buffer[4] = d1;
    buffer[5] = d2;
    let (s1, s2) = to_le(source);
    buffer[6] = s1;
    buffer[7] = s2;
    let (c1, c2) = to_le(calc_crc(&buffer[0..8]));
    buffer[8] = c1;
    buffer[9] = c2;
}

pub(crate) fn format_unconfirmed_user_data(
    is_master: bool,
    destination: u16,
    source: u16,
    payload: Payload,
    cursor: &mut WriteCursor,
) -> Result<(), WriteError> {
    format(
        ControlField::new(is_master, Function::PriUnconfirmedUserData),
        destination,
        source,
        Some(payload),
        cursor,
    )
}

fn format(
    control: ControlField,
    destination: u16,
    source: u16,
    payload: Option<Payload>,
    cursor: &mut WriteCursor,
) -> Result<(), WriteError> {
    fn format_payload(payload: Payload, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        // the first block contains the transport header
        let (first, remainder) = payload
            .app_data
            .np_split_at_no_error(constant::MAX_BLOCK_SIZE - 1);

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
                return Err(WriteError);
            }
            payload.app_data.len() as u8 + constant::MIN_HEADER_LENGTH_VALUE + 1
        }
        None => constant::MIN_HEADER_LENGTH_VALUE,
    };

    cursor.write_u8(constant::START1)?;
    cursor.write_u8(constant::START2)?;

    let header_start = cursor.position();

    cursor.write_u8(length)?;
    cursor.write_u8(control.to_u8())?;
    cursor.write_u16_le(destination)?;
    cursor.write_u16_le(source)?;
    cursor.write_u16_le(calc_crc_with_0564(cursor.written_since(header_start)?))?;

    match payload {
        Some(payload) => format_payload(payload, cursor),
        None => Ok(()),
    }
}

#[cfg(test)]
mod test {

    use super::super::test_data::*;
    use super::*;

    #[test]
    fn formats_ack() {
        let mut buffer = [0; constant::LINK_HEADER_LENGTH];
        format_header(
            ACK.header.control,
            ACK.header.destination.value(),
            ACK.header.source.value(),
            &mut buffer,
        );
        assert_eq!(buffer, ACK.bytes);
    }

    #[test]
    fn formats_unconfirmed_user_data() {
        let mut buffer = [0; constant::MAX_LINK_FRAME_LENGTH];
        let mut cursor = WriteCursor::new(&mut buffer);
        let start = cursor.position();
        format_unconfirmed_user_data(
            true,
            UNCONFIRMED_USER_DATA.header.destination.value(),
            UNCONFIRMED_USER_DATA.header.source.value(),
            Payload::new(0xC0, &UNCONFIRMED_USER_DATA.payload[1..]),
            &mut cursor,
        )
        .unwrap();
        assert_eq!(
            cursor.written_since(start).unwrap(),
            UNCONFIRMED_USER_DATA.bytes
        );
    }
}
