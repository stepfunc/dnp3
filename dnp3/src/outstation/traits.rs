use crate::app::enums::CommandStatus;
use crate::app::parse::count::CountSequence;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::outstation::database::Database;

/// Enumeration returned for cold/warm restart
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RestartDelay {
    /// corresponds to g51v1
    Seconds(u16),
    /// corresponds to g52v2
    Milliseconds(u16),
}

/// dynamic information required by the outstation from the user application
pub trait OutstationApplication: Sync + Send + 'static {
    /// The value returned by this method is used in conjunction with the `Delay Measurement`
    /// function code and returned in a g52v2 time delay object as part of a non-LAN time
    /// synchronization procedure.
    ///
    /// It represents the processing delay from receiving the request to sending the response.
    /// This parameter should almost always use the default value of zero as only an RTOS
    /// or bare metal system would have access to this level of timing. Modern hardware
    /// can almost always respond in less than 1 millisecond anyway.
    ///
    /// For more information, see IEEE-1815 2012, pg. 64
    fn get_processing_delay_ms(&self) -> u16 {
        0
    }

    /// Request that the outstation perform a cold restart (IEEE-1815 2012, pg. 58)
    ///
    /// If supported, return Some(RestartDelay) indicating how long the restart
    /// will take to complete
    ///
    /// returning None, will cause the outstation to return IIN.2 FUNCTION_NOT_SUPPORTED
    ///
    /// The outstation will not automatically restart. It is the responsibility of the user
    /// application to handle this request and take the appropriate acton.
    fn cold_restart(&mut self) -> Option<RestartDelay> {
        None
    }

    /// Request that the outstation perform a warm restart (IEEE-1815 2012, pg. 58)
    ///
    /// If supported, return Some(RestartDelay) indicating how long the restart
    /// will take to complete
    ///
    /// returning None, will cause the outstation to return IIN.2 FUNCTION_NOT_SUPPORTED
    ///
    /// The outstation will not automatically restart. It is the responsibility of the user
    /// application to handle this request and take the appropriate acton.
    fn warm_restart(&mut self) -> Option<RestartDelay> {
        None
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperateType {
    SelectBeforeOperate,
    DirectOperate,
    DirectOperateNoAck,
}

/// select, operate, or direct operate a control
pub trait ControlSupport<T> {
    /// select control, but do not operate
    fn select(&mut self, control: T, index: u16) -> CommandStatus;
    /// operate a control
    fn operate(
        &mut self,
        control: T,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus;
}

/// callbacks for handling controls
pub trait ControlHandler:
    ControlSupport<Group12Var1>
    + ControlSupport<Group41Var1>
    + ControlSupport<Group41Var2>
    + ControlSupport<Group41Var3>
    + ControlSupport<Group41Var4>
    + Sync
    + Send
    + 'static
{
    fn begin_fragment(&self) {}
    fn end_fragment(&self) {}
}

#[derive(Copy, Clone)]
pub struct DefaultOutstationApplication;

impl DefaultOutstationApplication {
    pub fn create() -> Box<dyn OutstationApplication> {
        Box::new(DefaultOutstationApplication)
    }
}

impl OutstationApplication for DefaultOutstationApplication {}

#[derive(Copy, Clone)]
pub struct DefaultControlHandler {
    status: CommandStatus,
}

impl DefaultControlHandler {
    pub fn create() -> Box<dyn ControlHandler> {
        Self::with_status(CommandStatus::NotSupported)
    }

    pub fn with_status(status: CommandStatus) -> Box<dyn ControlHandler> {
        Box::new(DefaultControlHandler { status })
    }
}

impl ControlHandler for DefaultControlHandler {}

impl ControlSupport<Group12Var1> for DefaultControlHandler {
    fn select(&mut self, _control: Group12Var1, _index: u16) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group12Var1,
        _index: u16,
        _op_type: OperateType,
        _database: &mut Database,
    ) -> CommandStatus {
        self.status
    }
}

impl ControlSupport<Group41Var1> for DefaultControlHandler {
    fn select(&mut self, _control: Group41Var1, _index: u16) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group41Var1,
        _index: u16,
        _op_type: OperateType,
        _database: &mut Database,
    ) -> CommandStatus {
        self.status
    }
}

impl ControlSupport<Group41Var2> for DefaultControlHandler {
    fn select(&mut self, _control: Group41Var2, _index: u16) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group41Var2,
        _index: u16,
        _op_type: OperateType,
        _database: &mut Database,
    ) -> CommandStatus {
        self.status
    }
}

impl ControlSupport<Group41Var3> for DefaultControlHandler {
    fn select(&mut self, _control: Group41Var3, _index: u16) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group41Var3,
        _index: u16,
        _op_type: OperateType,
        _database: &mut Database,
    ) -> CommandStatus {
        self.status
    }
}

impl ControlSupport<Group41Var4> for DefaultControlHandler {
    fn select(&mut self, _control: Group41Var4, _index: u16) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group41Var4,
        _index: u16,
        _op_type: OperateType,
        _database: &mut Database,
    ) -> CommandStatus {
        self.status
    }
}

trait HasCommandStatus {
    fn status(&self) -> CommandStatus;
    fn with_status(&self, status: CommandStatus) -> Self;
}

trait ControlSupportExt<T>: ControlSupport<T>
where
    T: FixedSizeVariation + HasCommandStatus,
{
    fn operate<I, F>(
        &mut self,
        seq: CountSequence<Prefix<I, T>>,
        op_type: OperateType,
        database: &mut Database,
        mut func: F,
    ) where
        F: FnMut(T, I),
        I: Index,
    {
        for item in seq.iter() {
            let status = {
                if item.value.status() == CommandStatus::Success {
                    ControlSupport::<T>::operate(
                        self,
                        item.value,
                        item.index.widen_to_u16(),
                        op_type,
                        database,
                    )
                } else {
                    CommandStatus::FormatError
                }
            };
            func(item.value.with_status(status), item.index)
        }
    }

    fn select<I, F>(&mut self, seq: CountSequence<Prefix<I, T>>, mut func: F)
    where
        F: FnMut(T, I),
        I: Index,
    {
        for item in seq.iter() {
            let status = {
                if item.value.status() == CommandStatus::Success {
                    ControlSupport::<T>::select(self, item.value, item.index.widen_to_u16())
                } else {
                    CommandStatus::FormatError
                }
            };
            func(item.value.with_status(status), item.index)
        }
    }
}
