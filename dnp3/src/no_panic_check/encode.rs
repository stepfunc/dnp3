// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Links the encoders for everything the master sends, and the link and transport framing the
//! master and the outstation share, with values and output buffers the compiler can't know.
//!
//! Each function marked `no_panic` here is one checked entry point. Its values are built from
//! the input outside it, so building them can allocate, and every variant of every enum can be
//! chosen by the input. Its output buffer is input-sized from 0 bytes to 16 MiB, so empty,
//! short, exact-fit and large buffers, and the 8- and 16-bit count and length limits, are all
//! reachable. It hands the result and the bytes written to `black_box`, so the encoding stays
//! in the check. The check proves only that no panic is reachable: the existing tests pin the
//! bytes.
//!
//! Checked, and what each covers:
//!
//! - [`link_frame`]: `format_header_only` and `format_data_frame`, with any header (including
//!   the link status request the transport writer sends) and payloads above the 249 byte maximum
//! - [`transport_segments`]: `format_segment` for every segment of a fragment, with the role,
//!   addresses and starting sequence number from the input. The loop that splits the fragment
//!   and marks the first and last segment is a copy of the one in `Writer::write`
//! - [`request_header`]: `start_request`, which starts every request and CONFIRM
//! - [`read_request`]: `ReadRequest::format`, which periodic polls and single reads use
//! - [`command_headers`]: `CommandHeaders::write`, for g12v1 and g41v1 to g41v4 with 8 and
//!   16-bit indexes, which commands with and without acknowledgment use
//! - [`one_shot`]: `OneShotRequest::write` for deadbands, generic requests (`Headers`, with read
//!   headers, freeze intervals and attributes), restarts and the file requests
//! - [`auto_task`]: `ResponseTask::write` for the automatic tasks (class scans for startup
//!   integrity and event scans are `Classes::write`, reached through [`read_request`])
//! - [`time_sync`] and [`file_read`]: the writers that the time sync and file read tasks choose
//!   between, by state
//!
//! Not checked: the dispatch from `ReadTask`, `NonReadTask`, `OneShot`, `CommandTask`,
//! `CommandNoAckTask`, `TimeSyncTask` and `FileReadTask` to these writers, each a match that
//! only picks one; asynchronous I/O; logging of what's sent; task scheduling and promises; the
//! outstation's responses. File requests whose objects have constant fields (close, get info,
//! read) are covered for the values the master sends, not for every g70 object.

use std::hint::black_box;

use scursor::WriteCursor;

use crate::app::attr::{AttrSet, FloatType, OwnedAttrValue, OwnedAttribute};
use crate::app::control::{
    CommandStatus, ControlCode, Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4,
    OpType, TripCloseCode,
};
use crate::app::format::write::{start_request, HeaderWriter};
use crate::app::variations::{Group50Var1, Group50Var3};
use crate::app::{
    ControlField as AppControlField, FunctionCode, PermissionSet, Permissions, Timestamp, Variation,
};
use crate::link::constant::MAX_APP_BYTES_PER_FRAME;
use crate::link::format::{format_data_frame, format_header_only, Payload};
use crate::link::header::{AnyAddress, ControlField, Header as LinkHeader};
use crate::link::{EndpointAddress, LinkRole};
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::deadbands::DeadBandsRequest;
use crate::master::tasks::empty_response::EmptyResponseRequest;
use crate::master::tasks::file::authenticate::AuthFileRequest;
use crate::master::tasks::file::close::CloseFileRequest;
use crate::master::tasks::file::get_info::GetFileInfoRequest;
use crate::master::tasks::file::open::OpenFileRequest;
use crate::master::tasks::file::write_block::WriteBlockRequest;
use crate::master::tasks::file::{write_auth, write_close, write_open, write_read};
use crate::master::tasks::restart::RestartType;
use crate::master::tasks::{OneShotRequest, ResponseTask};
use crate::master::{
    AuthKey, BlockNumber, Classes, CommandBuilder, CommandHeaders, CommandSupport, DeadBandHeader,
    EventClasses, FileCredentials, FileHandle, FileMode, Headers, ReadHeader, ReadRequest,
};
use crate::outstation::FreezeInterval;
use crate::transport::real::sequence::Sequence;
use crate::transport::real::writer::{format_segment, Position};

/// Values drawn from the input in order, zero once it runs out
struct Source<'a>(&'a [u8]);

impl Source<'_> {
    fn u8(&mut self) -> u8 {
        match self.0 {
            [first, rest @ ..] => {
                self.0 = rest;
                *first
            }
            [] => 0,
        }
    }

    fn bool(&mut self) -> bool {
        self.u8() & 0x01 != 0
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes([self.u8(), self.u8()])
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes([self.u8(), self.u8(), self.u8(), self.u8()])
    }

    fn u64(&mut self) -> u64 {
        u64::from_le_bytes([
            self.u8(),
            self.u8(),
            self.u8(),
            self.u8(),
            self.u8(),
            self.u8(),
            self.u8(),
            self.u8(),
        ])
    }

    /// a length from 0 to 16 MiB
    fn len(&mut self) -> usize {
        let len = u32::from_le_bytes([self.u8(), self.u8(), self.u8(), 0]);
        usize::try_from(len).unwrap_or(0)
    }

    fn bytes(&mut self) -> Vec<u8> {
        let len = self.len();
        (0..len).map(|_| self.u8()).collect()
    }

    fn string(&mut self) -> String {
        String::from_utf8_lossy(&self.bytes()).into_owned()
    }

    fn many<T>(&mut self, mut item: impl FnMut(&mut Self) -> T) -> Vec<T> {
        let len = self.len();
        (0..len).map(|_| item(self)).collect()
    }

    /// an output buffer of any length the encoders could be given
    fn buffer(&mut self) -> Vec<u8> {
        vec![0; self.len()]
    }
}

/// hand `value`, and anything it refers to, to the optimizer as used
fn used<T>(value: T) {
    black_box(value);
}

/// hand everything an encoder produced to the optimizer as used
fn consume<T>(result: T, cursor: &WriteCursor) {
    used(result);
    used(cursor.written());
}

/// run `write` on a writer over `buffer` and consume what it produced
fn encode<E>(buffer: &mut [u8], write: impl FnOnce(&mut HeaderWriter) -> Result<(), E>) {
    let mut cursor = WriteCursor::new(buffer);
    let result = write(&mut HeaderWriter::new(&mut cursor));
    consume(result, &cursor);
}

#[no_panic::no_panic]
fn link_frame(header: LinkHeader, payload: Option<Payload>, buffer: &mut [u8]) {
    let mut cursor = WriteCursor::new(buffer);
    let result = match payload {
        Some(payload) => format_data_frame(header, payload, &mut cursor),
        None => format_header_only(header, &mut cursor),
    };
    used(result);
}

/// the segments of `fragment`, split and marked as `Writer::write` does
#[no_panic::no_panic]
fn transport_segments(
    role: LinkRole,
    local: EndpointAddress,
    destination: EndpointAddress,
    seq: &mut Sequence,
    fragment: &[u8],
    buffer: &mut [u8],
) {
    let mut chunks = crate::util::chunks::<_, { MAX_APP_BYTES_PER_FRAME }>(fragment).peekable();
    let mut first = true;
    while let Some(chunk) = chunks.next() {
        let last = chunks.peek().is_none();
        let mut cursor = WriteCursor::new(buffer);
        let position = Position { first, last };
        used(format_segment(
            role,
            local,
            destination,
            seq,
            position,
            chunk,
            &mut cursor,
        ));
        first = false;
    }
    used(seq.value());
}

#[no_panic::no_panic]
fn request_header(control: AppControlField, function: FunctionCode, buffer: &mut [u8]) {
    let mut cursor = WriteCursor::new(buffer);
    let result = start_request(control, function, &mut cursor).map(|_| ());
    consume(result, &cursor);
}

#[no_panic::no_panic]
fn read_request(request: &ReadRequest, buffer: &mut [u8]) {
    encode(buffer, |writer| request.format(writer));
}

#[no_panic::no_panic]
fn command_headers(headers: &CommandHeaders, buffer: &mut [u8]) {
    encode(buffer, |writer| headers.write(writer));
}

#[no_panic::no_panic]
fn one_shot<R: OneShotRequest>(request: &R, buffer: &mut [u8]) {
    encode(buffer, |writer| request.write(writer));
}

#[no_panic::no_panic]
fn auto_task(task: &AutoTask, buffer: &mut [u8]) {
    encode(buffer, |writer| task.write(writer));
}

/// the writes of the time sync task's two WRITE states
#[no_panic::no_panic]
fn time_sync(absolute: bool, time: Timestamp, buffer: &mut [u8]) {
    encode(buffer, |writer| {
        if absolute {
            writer.write_count_of_one(Group50Var1 { time })
        } else {
            writer.write_count_of_one(Group50Var3 { time })
        }
    });
}

/// the state a file read task writes from
enum FileReadState {
    GetAuth(FileCredentials),
    Open {
        name: String,
        auth_key: AuthKey,
        max_block_size: u16,
    },
    Read(FileHandle, BlockNumber),
    Close(FileHandle),
}

/// the writes of the file read task's states, with the arguments it passes
#[no_panic::no_panic]
fn file_read(state: &FileReadState, buffer: &mut [u8]) {
    encode(buffer, |writer| match state {
        FileReadState::GetAuth(credentials) => write_auth(credentials, writer),
        FileReadState::Open {
            name,
            auth_key,
            max_block_size,
        } => write_open(
            name,
            *auth_key,
            Permissions::default(),
            0,
            FileMode::Read,
            *max_block_size,
            writer,
        ),
        FileReadState::Read(handle, block) => write_read(*handle, *block, writer),
        FileReadState::Close(handle) => write_close(*handle, writer),
    });
}

fn link_header(source: &mut Source) -> LinkHeader {
    LinkHeader::new(
        ControlField::from(source.u8()),
        AnyAddress::from(source.u16()),
        AnyAddress::from(source.u16()),
    )
}

fn link_role(source: &mut Source) -> LinkRole {
    match source.u8() % 3 {
        0 => LinkRole::Master,
        1 => LinkRole::Outstation {
            self_address: false,
        },
        _ => LinkRole::Outstation { self_address: true },
    }
}

/// a defined variation, or g0v0, which names no object
fn variation(source: &mut Source) -> Variation {
    Variation::lookup(source.u8(), source.u8()).unwrap_or(Variation::Group0(0))
}

fn read_header(source: &mut Source) -> ReadHeader {
    let variation = variation(source);
    match source.u8() % 5 {
        0 => ReadHeader::one_byte_range(variation, source.u8(), source.u8()),
        1 => ReadHeader::two_byte_range(variation, source.u16(), source.u16()),
        2 => ReadHeader::all_objects(variation),
        3 => ReadHeader::one_byte_limited_count(variation, source.u8()),
        _ => ReadHeader::two_byte_limited_count(variation, source.u16()),
    }
}

fn event_classes(source: &mut Source) -> EventClasses {
    EventClasses::new(source.bool(), source.bool(), source.bool())
}

fn classes(source: &mut Source) -> Classes {
    Classes::new(source.bool(), event_classes(source))
}

fn read(source: &mut Source) -> ReadRequest {
    match source.u8() % 3 {
        0 => ReadRequest::SingleHeader(read_header(source)),
        1 => ReadRequest::ClassScan(classes(source)),
        _ => ReadRequest::MultipleHeader(source.many(read_header)),
    }
}

fn attr_value(source: &mut Source) -> OwnedAttrValue {
    match source.u8() % 8 {
        0 => OwnedAttrValue::VisibleString(source.string()),
        1 => OwnedAttrValue::UnsignedInt(source.u32()),
        2 => OwnedAttrValue::SignedInt(source.u32().cast_signed()),
        3 => OwnedAttrValue::FloatingPoint(FloatType::F32(f32::from_bits(source.u32()))),
        4 => OwnedAttrValue::FloatingPoint(FloatType::F64(f64::from_bits(source.u64()))),
        5 => OwnedAttrValue::OctetString(source.bytes()),
        6 => OwnedAttrValue::Dnp3Time(Timestamp::new(source.u64())),
        _ => OwnedAttrValue::BitString(source.bytes()),
    }
}

fn attribute(source: &mut Source) -> OwnedAttribute {
    let set = if source.bool() {
        AttrSet::Default
    } else {
        AttrSet::Private(source.u8())
    };
    OwnedAttribute::new(set, source.u8(), attr_value(source))
}

fn headers(source: &mut Source) -> Headers {
    let len = source.len();
    (0..len).fold(Headers::new(), |headers, _| {
        let variation = variation(source);
        match source.u8() % 7 {
            0 => headers.add_all_objects(variation),
            1 => headers.add_range_8(variation, source.u8(), source.u8()),
            2 => headers.add_range_16(variation, source.u16(), source.u16()),
            3 => headers.add_one_byte_limited_count(variation, source.u8()),
            4 => headers.add_two_byte_limited_count(variation, source.u16()),
            5 => headers.add_freeze_interval(FreezeInterval::new(
                Timestamp::new(source.u64()),
                source.u32(),
            )),
            _ => headers.add_attribute(attribute(source)),
        }
    })
}

/// a control code with each field defined or not, including the undefined values that alias a
/// defined one, which `ControlCode::from` never produces
fn control_code(source: &mut Source) -> ControlCode {
    let tcc = source.u8();
    let op_type = source.u8();
    ControlCode {
        tcc: if source.bool() {
            TripCloseCode::from(tcc)
        } else {
            TripCloseCode::Unknown(tcc)
        },
        clear: source.bool(),
        queue: source.bool(),
        op_type: if source.bool() {
            OpType::from(op_type)
        } else {
            OpType::Unknown(op_type)
        },
    }
}

fn status(source: &mut Source) -> CommandStatus {
    CommandStatus::from(source.u8())
}

/// add `command` with an 8 or 16-bit index
fn add<T>(builder: &mut CommandBuilder, source: &mut Source, command: T)
where
    CommandBuilder: CommandSupport<T>,
{
    if source.bool() {
        builder.add_u8(command, source.u8());
    } else {
        builder.add_u16(command, source.u16());
    }
}

fn commands(source: &mut Source) -> CommandHeaders {
    let mut builder = CommandBuilder::new();
    let len = source.len();
    for _ in 0..len {
        match source.u8() % 6 {
            0 => {
                let command = Group12Var1 {
                    code: control_code(source),
                    count: source.u8(),
                    on_time: source.u32(),
                    off_time: source.u32(),
                    status: status(source),
                };
                add(&mut builder, source, command);
            }
            1 => {
                let command = Group41Var1 {
                    value: source.u32().cast_signed(),
                    status: status(source),
                };
                add(&mut builder, source, command);
            }
            2 => {
                let command = Group41Var2 {
                    value: source.u16().cast_signed(),
                    status: status(source),
                };
                add(&mut builder, source, command);
            }
            3 => {
                let command = Group41Var3 {
                    value: f32::from_bits(source.u32()),
                    status: status(source),
                };
                add(&mut builder, source, command);
            }
            4 => {
                let command = Group41Var4 {
                    value: f64::from_bits(source.u64()),
                    status: status(source),
                };
                add(&mut builder, source, command);
            }
            _ => builder.finish_header(),
        }
    }
    builder.build()
}

fn dead_band_header(source: &mut Source) -> DeadBandHeader {
    match source.u8() % 6 {
        0 => DeadBandHeader::group34_var1_u8(source.many(|x| (x.u8(), x.u16()))),
        1 => DeadBandHeader::group34_var1_u16(source.many(|x| (x.u16(), x.u16()))),
        2 => DeadBandHeader::group34_var2_u8(source.many(|x| (x.u8(), x.u32()))),
        3 => DeadBandHeader::group34_var2_u16(source.many(|x| (x.u16(), x.u32()))),
        4 => DeadBandHeader::group34_var3_u8(source.many(|x| (x.u8(), f32::from_bits(x.u32())))),
        _ => DeadBandHeader::group34_var3_u16(source.many(|x| (x.u16(), f32::from_bits(x.u32())))),
    }
}

fn credentials(source: &mut Source) -> FileCredentials {
    FileCredentials {
        user_name: source.string(),
        password: source.string(),
    }
}

fn permission_set(source: &mut Source) -> PermissionSet {
    PermissionSet {
        execute: source.bool(),
        write: source.bool(),
        read: source.bool(),
    }
}

fn permissions(source: &mut Source) -> Permissions {
    Permissions {
        world: permission_set(source),
        group: permission_set(source),
        owner: permission_set(source),
    }
}

fn file_mode(source: &mut Source) -> FileMode {
    match source.u8() % 5 {
        0 => FileMode::Null,
        1 => FileMode::Read,
        2 => FileMode::Write,
        3 => FileMode::Append,
        _ => FileMode::Reserved(source.u16()),
    }
}

fn file_handle(source: &mut Source) -> FileHandle {
    FileHandle::new(source.u32())
}

fn block_number(source: &mut Source) -> BlockNumber {
    BlockNumber::new(source.u32())
}

fn auto(source: &mut Source) -> AutoTask {
    match source.u8() % 3 {
        0 => AutoTask::ClearRestartBit,
        1 => AutoTask::EnableUnsolicited(event_classes(source)),
        _ => AutoTask::DisableUnsolicited(event_classes(source)),
    }
}

fn file_read_state(source: &mut Source) -> FileReadState {
    match source.u8() % 4 {
        0 => FileReadState::GetAuth(credentials(source)),
        1 => FileReadState::Open {
            name: source.string(),
            auth_key: AuthKey::new(source.u32()),
            max_block_size: source.u16(),
        },
        2 => FileReadState::Read(file_handle(source), block_number(source)),
        _ => FileReadState::Close(file_handle(source)),
    }
}

/// every encoder, each given values and an output buffer chosen by the input
fn encoders(source: &mut Source) {
    let header = link_header(source);
    let payload = source.bool().then(|| (source.u8(), source.bytes()));
    let payload = payload
        .as_ref()
        .map(|(transport, data)| Payload::new(*transport, data));
    link_frame(header, payload, &mut source.buffer());

    let role = link_role(source);
    let local = EndpointAddress::raw(source.u16());
    let destination = EndpointAddress::raw(source.u16());
    let mut seq = Sequence::new(source.u8());
    let fragment = source.bytes();
    transport_segments(
        role,
        local,
        destination,
        &mut seq,
        &fragment,
        &mut source.buffer(),
    );

    let control = AppControlField::from(source.u8());
    let function = FunctionCode::from(source.u8()).unwrap_or(FunctionCode::Confirm);
    request_header(control, function, &mut source.buffer());

    read_request(&read(source), &mut source.buffer());
    command_headers(&commands(source), &mut source.buffer());
    auto_task(&auto(source), &mut source.buffer());
    time_sync(
        source.bool(),
        Timestamp::new(source.u64()),
        &mut source.buffer(),
    );
    file_read(&file_read_state(source), &mut source.buffer());

    let dead_bands = DeadBandsRequest::new(source.many(dead_band_header));
    one_shot(&dead_bands, &mut source.buffer());
    let function = FunctionCode::from(source.u8()).unwrap_or(FunctionCode::Write);
    let generic = EmptyResponseRequest::new(function, headers(source));
    one_shot(&generic, &mut source.buffer());
    let restart = if source.bool() {
        RestartType::ColdRestart
    } else {
        RestartType::WarmRestart
    };
    one_shot(&restart, &mut source.buffer());
    let auth = AuthFileRequest {
        credentials: credentials(source),
    };
    one_shot(&auth, &mut source.buffer());
    let open = OpenFileRequest {
        file_name: source.string(),
        auth_key: AuthKey::new(source.u32()),
        file_size: source.u32(),
        file_mode: file_mode(source),
        permissions: permissions(source),
        max_block_size: source.u16(),
    };
    one_shot(&open, &mut source.buffer());
    let close = CloseFileRequest {
        handle: file_handle(source),
    };
    one_shot(&close, &mut source.buffer());
    let block = WriteBlockRequest {
        handle: file_handle(source),
        block_number: block_number(source),
        block_data: source.bytes(),
    };
    one_shot(&block, &mut source.buffer());
    let info = GetFileInfoRequest::new(source.string());
    one_shot(&info, &mut source.buffer());
}

#[test]
fn encoders_with_unknown_input() {
    let data = black_box(super::input());
    encoders(&mut Source(&data));
}
