use crate::decode::LinkDecodeLevel;
use crate::link::constant;
use crate::link::crc::{calc_crc, calc_crc_with_0564};
use crate::link::display::LinkDisplay;
use crate::link::error::LogicError;
use crate::link::header::Header;
use crate::util::slice_ext::SliceExtNoPanic;

use scursor::WriteCursor;

use crate::util::BadWrite;

impl From<BadWrite> for LogicError {
    fn from(_: BadWrite) -> Self {
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

// A view that we can use for tx logging
pub(crate) struct FrameData<'a> {
    pub(crate) frame: &'a [u8],
    header: Header,
    payload_only: &'a [u8],
}

impl FrameData<'_> {
    pub(crate) fn to_link_display(&self, level: LinkDecodeLevel) -> LinkDisplay<'_> {
        LinkDisplay::new(self.header, self.payload_only, level)
    }
}

/// The 10 bytes of a link header announcing `length` (the header fields plus any user data)
pub(crate) fn header_bytes(header: Header, length: u8) -> [u8; constant::LINK_HEADER_LENGTH] {
    let ctrl = header.control.to_u8();
    let [d0, d1] = header.destination.value().to_le_bytes();
    let [s0, s1] = header.source.value().to_le_bytes();
    let [c0, c1] = calc_crc_with_0564(&[length, ctrl, d0, d1, s0, s1]).to_le_bytes();
    [
        constant::START1,
        constant::START2,
        length,
        ctrl,
        d0,
        d1,
        s0,
        s1,
        c0,
        c1,
    ]
}

pub(crate) fn format_header_only<'a>(
    header: Header,
    cursor: &'a mut WriteCursor,
) -> Result<FrameData<'a>, BadWrite> {
    format_frame(header, None, cursor)
}

pub(crate) fn format_data_frame<'a>(
    header: Header,
    payload: Payload,
    cursor: &'a mut WriteCursor,
) -> Result<FrameData<'a>, BadWrite> {
    format_frame(header, Some(payload), cursor)
}

// generic frame formatting function
fn format_frame<'a>(
    header: Header,
    payload: Option<Payload>,
    cursor: &'a mut WriteCursor,
) -> Result<FrameData<'a>, BadWrite> {
    fn format_payload(payload: Payload, cursor: &mut WriteCursor) -> Result<(), BadWrite> {
        // the first block contains the transport header
        let (first, remainder) = payload
            .app_data
            .np_split_at_no_error(constant::MAX_BLOCK_SIZE - 1);

        // write the first block
        let begin_first_block = cursor.position();
        cursor.write_u8(payload.transport)?;
        cursor.write_bytes(first)?;
        cursor.write_u16_le(calc_crc(cursor.written_since(begin_first_block)?))?;

        // write the remaining blocks
        for block in remainder.chunks(constant::MAX_BLOCK_SIZE) {
            let start_block = cursor.position();
            cursor.write_bytes(block)?;
            cursor.write_u16_le(calc_crc(cursor.written_since(start_block)?))?;
        }

        Ok(())
    }

    let length: u8 = match payload {
        Some(payload) => {
            if payload.app_data.len() > constant::MAX_APP_BYTES_PER_FRAME {
                return Err(BadWrite);
            }
            // the header fields, the transport header, and the data
            u8::try_from(payload.app_data.len())
                .ok()
                .and_then(|len| len.checked_add(constant::MIN_HEADER_LENGTH_VALUE))
                .and_then(|len| len.checked_add(1))
                .ok_or(BadWrite)?
        }
        None => constant::MIN_HEADER_LENGTH_VALUE,
    };

    let start = cursor.position();

    cursor.write_bytes(&header_bytes(header, length))?;

    let end_header = cursor.position();

    match payload {
        Some(payload) => {
            format_payload(payload, cursor)?;
            Ok(FrameData {
                header,
                frame: cursor.written_since(start)?,
                payload_only: cursor.written_since(end_header)?,
            })
        }
        None => Ok(FrameData {
            header,
            frame: cursor.written_since(start)?,
            payload_only: &[],
        }),
    }
}

#[cfg(test)]
mod test {
    use super::super::test_data::*;
    use super::*;

    #[test]
    fn formats_ack() {
        assert_eq!(
            header_bytes(ACK.header, constant::MIN_HEADER_LENGTH_VALUE),
            ACK.bytes
        );
    }

    #[test]
    fn formats_unconfirmed_user_data() {
        let mut buffer = [0; constant::MAX_LINK_FRAME_LENGTH];
        let mut cursor = WriteCursor::new(&mut buffer);
        let start = cursor.position();
        format_data_frame(
            Header::unconfirmed_user_data(
                true,
                UNCONFIRMED_USER_DATA.header.destination,
                UNCONFIRMED_USER_DATA.header.source,
            ),
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
