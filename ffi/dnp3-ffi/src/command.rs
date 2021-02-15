use crate::ffi;
use dnp3::app::types::ControlCode;
use dnp3::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use dnp3::app::{OpType, TripCloseCode};

#[derive(Clone)]
enum CommandHeaderElement {
    G12V1(Group12Var1),
    G41V1(Group41Var1),
    G41V2(Group41Var2),
    G41V3(Group41Var3),
    G41V4(Group41Var4),
}

impl From<&ffi::ControlCode> for ControlCode {
    fn from(from: &ffi::ControlCode) -> Self {
        Self {
            tcc: match from.tcc() {
                ffi::TripCloseCode::Nul => TripCloseCode::Nul,
                ffi::TripCloseCode::Close => TripCloseCode::Close,
                ffi::TripCloseCode::Trip => TripCloseCode::Trip,
                ffi::TripCloseCode::Reserved => TripCloseCode::Reserved,
            },
            clear: from.clear(),
            queue: from.queue(),
            op_type: match from.op_type() {
                ffi::OpType::Nul => OpType::Nul,
                ffi::OpType::PulseOn => OpType::PulseOn,
                ffi::OpType::PulseOff => OpType::PulseOff,
                ffi::OpType::LatchOn => OpType::LatchOn,
                ffi::OpType::LatchOff => OpType::LatchOff,
            },
        }
    }
}

impl From<ffi::G12v1> for CommandHeaderElement {
    fn from(from: ffi::G12v1) -> Self {
        Self::G12V1(Group12Var1::new(
            (&from.code).into(),
            from.count,
            from.on_time,
            from.off_time,
        ))
    }
}

impl From<Group41Var1> for CommandHeaderElement {
    fn from(from: Group41Var1) -> Self {
        Self::G41V1(from)
    }
}

impl From<Group41Var2> for CommandHeaderElement {
    fn from(from: Group41Var2) -> Self {
        Self::G41V2(from)
    }
}

impl From<Group41Var3> for CommandHeaderElement {
    fn from(from: Group41Var3) -> Self {
        Self::G41V3(from)
    }
}

impl From<Group41Var4> for CommandHeaderElement {
    fn from(from: Group41Var4) -> Self {
        Self::G41V4(from)
    }
}

#[derive(Clone)]
enum CommandHeader {
    U8(u8, CommandHeaderElement),
    U16(u16, CommandHeaderElement),
}

impl CommandHeader {
    fn u8(idx: u8, el: CommandHeaderElement) -> Self {
        Self::U8(idx, el)
    }

    fn u16(idx: u16, el: CommandHeaderElement) -> Self {
        Self::U16(idx, el)
    }
}

#[derive(Clone)]
pub struct Command {
    headers: Vec<CommandHeader>,
}

impl Command {
    fn new() -> Self {
        Self {
            headers: Vec::new(),
        }
    }

    fn push(&mut self, header: CommandHeader) {
        self.headers.push(header);
    }

    pub(crate) fn build(self) -> dnp3::master::request::CommandHeaders {
        let mut builder = dnp3::master::request::CommandBuilder::new();

        for header in self.headers {
            match header {
                CommandHeader::U8(idx, el) => match el {
                    CommandHeaderElement::G12V1(el) => builder.add_u8_header(el, idx),
                    CommandHeaderElement::G41V1(el) => builder.add_u8_header(el, idx),
                    CommandHeaderElement::G41V2(el) => builder.add_u8_header(el, idx),
                    CommandHeaderElement::G41V3(el) => builder.add_u8_header(el, idx),
                    CommandHeaderElement::G41V4(el) => builder.add_u8_header(el, idx),
                },
                CommandHeader::U16(idx, el) => match el {
                    CommandHeaderElement::G12V1(el) => builder.add_u16_header(el, idx),
                    CommandHeaderElement::G41V1(el) => builder.add_u16_header(el, idx),
                    CommandHeaderElement::G41V2(el) => builder.add_u16_header(el, idx),
                    CommandHeaderElement::G41V3(el) => builder.add_u16_header(el, idx),
                    CommandHeaderElement::G41V4(el) => builder.add_u16_header(el, idx),
                },
            }
        }

        builder.build()
    }
}

pub unsafe fn command_new() -> *mut Command {
    let command = Box::new(Command::new());
    Box::into_raw(command)
}

pub unsafe fn command_destroy(command: *mut Command) {
    if !command.is_null() {
        Box::from_raw(command);
    }
}

pub unsafe fn command_add_u8_g12v1(command: *mut Command, idx: u8, header: ffi::G12v1) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u8(idx, header.into()));
    }
}

pub unsafe fn command_add_u16_g12v1(command: *mut Command, idx: u16, header: ffi::G12v1) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u16(idx, header.into()));
    }
}

pub unsafe fn command_add_u8_g41v1(command: *mut Command, idx: u8, value: i32) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u8(idx, Group41Var1::new(value).into()));
    }
}

pub unsafe fn command_add_u16_g41v1(command: *mut Command, idx: u16, value: i32) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u16(idx, Group41Var1::new(value).into()));
    }
}

pub unsafe fn command_add_u8_g41v2(command: *mut Command, idx: u8, value: i16) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u8(idx, Group41Var2::new(value).into()));
    }
}

pub unsafe fn command_add_u16_g41v2(command: *mut Command, idx: u16, value: i16) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u16(idx, Group41Var2::new(value).into()));
    }
}

pub unsafe fn command_add_u8_g41v3(command: *mut Command, idx: u8, value: f32) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u8(idx, Group41Var3::new(value).into()));
    }
}

pub unsafe fn command_add_u16_g41v3(command: *mut Command, idx: u16, value: f32) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u16(idx, Group41Var3::new(value).into()));
    }
}

pub unsafe fn command_add_u8_g41v4(command: *mut Command, idx: u8, value: f64) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u8(idx, Group41Var4::new(value).into()));
    }
}

pub unsafe fn command_add_u16_g41v4(command: *mut Command, idx: u16, value: f64) {
    if let Some(command) = command.as_mut() {
        command.push(CommandHeader::u16(idx, Group41Var4::new(value).into()));
    }
}
