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

impl From<ffi::G12v1> for Group12Var1 {
    fn from(x: ffi::G12v1) -> Self {
        Group12Var1::new(x.code.into(), x.count, x.on_time, x.off_time)
    }
}

// Commands is just a handle to a CommandBuilder
pub type Commands = dnp3::master::CommandBuilder;

pub unsafe fn commands_new() -> *mut Commands {
    Box::into_raw(Box::new(CommandBuilder::new()))
}

pub unsafe fn commands_destroy(commands: *mut Commands) {
    if !commands.is_null() {
        Box::from_raw(commands);
    }
}

pub unsafe fn commands_finish_header(commands: *mut crate::Commands) {
    if let Some(commands) = commands.as_mut() {
        commands.finish_header();
    }
}

pub unsafe fn commands_add_g12v1_u8(commands: *mut Commands, idx: u8, value: ffi::G12v1) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group12Var1>::add_u8(commands, value.into(), idx);
    }
}

pub unsafe fn commands_add_g12v1_u16(commands: *mut Commands, idx: u16, value: ffi::G12v1) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group12Var1>::add_u16(commands, value.into(), idx);
    }
}

pub unsafe fn commands_add_g41v1_u8(commands: *mut Commands, idx: u8, value: i32) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var1>::add_u8(commands, Group41Var1::new(value), idx);
    }
}

pub unsafe fn commands_add_g41v1_u16(commands: *mut Commands, idx: u16, value: i32) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var1>::add_u16(commands, Group41Var1::new(value), idx);
    }
}

pub unsafe fn commands_add_g41v2_u8(commands: *mut Commands, idx: u8, value: i16) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var2>::add_u8(commands, Group41Var2::new(value), idx);
    }
}

pub unsafe fn commands_add_g41v2_u16(commands: *mut Commands, idx: u16, value: i16) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var2>::add_u16(commands, Group41Var2::new(value), idx);
    }
}

pub unsafe fn commands_add_g41v3_u8(commands: *mut Commands, idx: u8, value: f32) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var3>::add_u8(commands, Group41Var3::new(value), idx);
    }
}

pub unsafe fn commands_add_g41v3_u16(commands: *mut Commands, idx: u16, value: f32) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var3>::add_u16(commands, Group41Var3::new(value), idx);
    }
}

pub unsafe fn commands_add_g41v4_u8(commands: *mut Commands, idx: u8, value: f64) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var4>::add_u8(commands, Group41Var4::new(value), idx);
    }
}

pub unsafe fn commands_add_g41v4_u16(commands: *mut Commands, idx: u16, value: f64) {
    if let Some(commands) = commands.as_mut() {
        CommandSupport::<Group41Var4>::add_u16(commands, Group41Var4::new(value), idx);
    }
}
