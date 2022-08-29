use dnp3::app::control::*;
use dnp3::master::*;

use crate::ffi;

impl From<ffi::ControlCode> for ControlCode {
    fn from(from: ffi::ControlCode) -> Self {
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

impl From<ffi::Group12Var1> for Group12Var1 {
    fn from(x: ffi::Group12Var1) -> Self {
        Group12Var1::new(x.code.into(), x.count, x.on_time, x.off_time)
    }
}

// Commands is just a handle to a CommandBuilder
pub type CommandSet = dnp3::master::CommandBuilder;

pub unsafe fn command_set_create() -> *mut CommandSet {
    Box::into_raw(Box::new(CommandBuilder::new()))
}

pub unsafe fn command_set_destroy(commands: *mut CommandSet) {
    if !commands.is_null() {
        drop(Box::from_raw(commands));
    }
}

pub unsafe fn command_set_finish_header(commands: *mut CommandSet) {
    if let Some(commands) = commands.as_mut() {
        commands.finish_header();
    }
}

pub unsafe fn command_set_add_g12_v1_u8(
    commands: *mut CommandSet,
    idx: u8,
    value: ffi::Group12Var1,
) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group12Var1>::add_u8(commands, value.into(), idx);
    }
}

pub unsafe fn command_set_add_g12_v1_u16(
    commands: *mut CommandSet,
    idx: u16,
    value: ffi::Group12Var1,
) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group12Var1>::add_u16(commands, value.into(), idx);
    }
}

pub unsafe fn command_set_add_g41_v1_u8(commands: *mut CommandSet, idx: u8, value: i32) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var1>::add_u8(commands, Group41Var1::new(value), idx);
    }
}

pub unsafe fn command_set_add_g41_v1_u16(commands: *mut CommandSet, idx: u16, value: i32) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var1>::add_u16(commands, Group41Var1::new(value), idx);
    }
}

pub unsafe fn command_set_add_g41_v2_u8(commands: *mut CommandSet, idx: u8, value: i16) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var2>::add_u8(commands, Group41Var2::new(value), idx);
    }
}

pub unsafe fn command_set_add_g41_v2_u16(commands: *mut CommandSet, idx: u16, value: i16) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var2>::add_u16(commands, Group41Var2::new(value), idx);
    }
}

pub unsafe fn command_set_add_g41_v3_u8(commands: *mut CommandSet, idx: u8, value: f32) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var3>::add_u8(commands, Group41Var3::new(value), idx);
    }
}

pub unsafe fn command_set_add_g41_v3_u16(commands: *mut CommandSet, idx: u16, value: f32) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var3>::add_u16(commands, Group41Var3::new(value), idx);
    }
}

pub unsafe fn command_set_add_g41_v4_u8(commands: *mut CommandSet, idx: u8, value: f64) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var4>::add_u8(commands, Group41Var4::new(value), idx);
    }
}

pub unsafe fn command_set_add_g41_v4_u16(commands: *mut CommandSet, idx: u16, value: f64) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var4>::add_u16(commands, Group41Var4::new(value), idx);
    }
}
