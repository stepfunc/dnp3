use crate::app::parse::count::CountSequence;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::variations::Group50Var2;
use crate::app::RequestHeader;
use crate::app::Sequence;
use crate::app::{control::*, Timestamp};
use crate::app::{FunctionCode, MaybeAsync};
use crate::outstation::database::DatabaseHandle;

/// Application-controlled IIN bits
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct ApplicationIin {
    /// IIN1.4: Time synchronization is required
    pub need_time: bool,
    /// IIN1.5: Some output points are in local mode
    pub local_control: bool,
    /// IIN1.6: Device trouble
    pub device_trouble: bool,
    /// IIN2.5 Configuration corrupt
    pub config_corrupt: bool,
}

/// Enumeration returned for cold/warm restart
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RestartDelay {
    /// corresponds to g51v1
    Seconds(u16),
    /// corresponds to g52v2
    Milliseconds(u16),
}

/// Enum describing the result of an operation
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RequestError {
    /// outstation supports this operation, but the parameter(s) are nonsensical.
    ParameterError,
    /// outstation does not support this operation
    NotSupported,
}

/// Outstation connection state for connection-oriented transports, e.g. TCP
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connected to the master
    Connected,
    /// Disconnected from the master
    Disconnected,
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

    /// Handle a write of the absolute time.
    ///
    /// This is used during time synchronization procedures.
    #[allow(unused_variables)]
    fn write_absolute_time(&mut self, time: Timestamp) -> Result<(), RequestError> {
        Err(RequestError::NotSupported)
    }

    /// Returns the application-controlled IIN bits
    fn get_application_iin(&self) -> ApplicationIin {
        ApplicationIin::default()
    }

    /// Request that the outstation perform a cold restart (IEEE-1815 2012, pg. 58)
    ///
    /// If supported, return Some(RestartDelay) indicating how long the restart
    /// will take to complete
    ///
    /// returning None, will cause the outstation to return IIN2.0 NO_FUNC_CODE_SUPPORT
    ///
    /// The outstation will not automatically restart. It is the responsibility of the user
    /// application to handle this request and take the appropriate action.
    fn cold_restart(&mut self) -> Option<RestartDelay> {
        None
    }

    /// Request that the outstation perform a warm restart (IEEE-1815 2012, pg. 58)
    ///
    /// If supported, return Some(RestartDelay) indicating how long the restart
    /// will take to complete
    ///
    /// returning None, will cause the outstation to return IIN2.0 NO_FUNC_CODE_SUPPORT
    ///
    /// The outstation will not automatically restart. It is the responsibility of the user
    /// application to handle this request and take the appropriate action.
    fn warm_restart(&mut self) -> Option<RestartDelay> {
        None
    }

    /// Perform a counter freeze operation
    #[allow(unused_variables)]
    fn freeze_counter(
        &mut self,
        indices: FreezeIndices,
        freeze_type: FreezeType,
        database: &mut DatabaseHandle,
    ) -> Result<(), RequestError> {
        Err(RequestError::NotSupported)
    }

    /// Controls outstation support for writing group 34, analog input dead-bands
    ///
    /// Returning false, indicates that the writes to group34 should not be processed and requests to
    /// do so should be rejected with IIN2.NO_FUNC_CODE_SUPPORT
    ///
    /// Returning true will allow the request to process the actual values with a sequence of calls:
    ///
    /// 1) A single call to [`Self::begin_write_analog_dead_bands`]
    /// 2) Zero or more calls to [`Self::write_analog_dead_band`]
    /// 3) A single call to [`Self::end_write_analog_dead_bands`]
    fn support_write_analog_dead_bands(&mut self) -> bool {
        false
    }

    /// Called when the outstation begins processing a header to write analog dead-bands
    fn begin_write_analog_dead_bands(&mut self) {}

    /// Called for each analog dead-band in the write request where an analog input is defined
    /// at the specified index.
    ///
    /// The dead-band is automatically updated in the database. This callback allows application code
    /// to persist the modified value to non-volatile memory if desired
    #[allow(unused_variables)]
    fn write_analog_dead_band(&mut self, index: u16, dead_band: f64) {}

    /// Called when the outstation completes processing a header to write analog dead-bands
    ///
    /// Multiple dead-bands changes can be accumulated in calls to [`Self::write_analog_dead_band`] and
    /// then be processed as a batch in this method.
    fn end_write_analog_dead_bands(&mut self) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }
}

/// enumeration describing how the outstation processed a broadcast request
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BroadcastAction {
    /// Outstation processed the broadcast
    Processed,
    /// Outstation ignored the broadcast message b/c it is disabled by configuration
    IgnoredByConfiguration,
    /// Outstation was unable to parse the object headers and ignored the request
    BadObjectHeaders,
    /// Outstation ignore the broadcast message b/c the function is not supported via Broadcast
    UnsupportedFunction(FunctionCode),
}

/// Informational callbacks that the outstation doesn't rely on to function,
/// but may be useful to certain applications to assess the health of the communication
/// or to count statistics
pub trait OutstationInformation: Sync + Send + 'static {
    /// called when a request is processed from the IDLE state
    fn process_request_from_idle(&mut self, _header: RequestHeader) {}
    /// called when a broadcast request is received by the outstation
    fn broadcast_received(&mut self, _function: FunctionCode, _action: BroadcastAction) {}
    /// outstation has begun waiting for a solicited confirm
    fn enter_solicited_confirm_wait(&mut self, _ecsn: Sequence) {}
    /// failed to receive a solicited confirm before the timeout occurred
    fn solicited_confirm_timeout(&mut self, _ecsn: Sequence) {}
    /// received the expected confirm
    fn solicited_confirm_received(&mut self, _ecsn: Sequence) {}
    /// received a new request while waiting for a solicited confirm, aborting the response series
    fn solicited_confirm_wait_new_request(&mut self) {}
    /// received a solicited confirm with the wrong sequence number
    fn wrong_solicited_confirm_seq(&mut self, _ecsn: Sequence, _seq: Sequence) {}
    /// received a confirm when not expecting one
    fn unexpected_confirm(&mut self, _unsolicited: bool, _seq: Sequence) {}
    /// outstation has begun waiting for an unsolicited confirm
    fn enter_unsolicited_confirm_wait(&mut self, _ecsn: Sequence) {}
    /// failed to receive an unsolicited confirm before the timeout occurred
    fn unsolicited_confirm_timeout(&mut self, _ecsn: Sequence, _retry: bool) {}
    /// master confirmed and unsolicited message
    fn unsolicited_confirmed(&mut self, _ecsn: Sequence) {}
    /// master cleared the restart IIN bit
    fn clear_restart_iin(&mut self) {}
}

/// enumeration describing how the master requested the control operation
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OperateType {
    /// control point was properly selected before the operate request
    SelectBeforeOperate,
    /// operate the control via a DirectOperate request
    DirectOperate,
    /// operate the control via a DirectOperateNoAck request
    DirectOperateNoAck,
}

/// select, operate, direct operate, or direct operate no-ack a control point
pub trait ControlSupport<T> {
    /// Select a control point, but do not operate. Implementors can think of
    /// this function ask the question "is this control supported"?
    ///
    /// Most implementations should not alter the database in this method. It
    /// is only provided in the event that some event counters reflected via the API
    /// get updated on SELECT, but this would be highly abnormal.
    ///
    /// arguments:
    ///
    /// * `control` value of the control
    /// * `index` index of the control
    /// * `database` reference to the database
    ///
    /// returns:
    ///
    /// `CommandStatus` enumeration returning either `CommandStatus::Success` if the operation is
    /// supported, or an error variant otherwise.
    fn select(&mut self, control: T, index: u16, database: &mut DatabaseHandle) -> CommandStatus;

    /// Operate a control point
    ///
    /// arguments:
    ///
    /// * `control` value of the control
    /// * `op_type` enumeration describing how the master requested the control operation. Most implementations
    ///             should just ignore this argument as the behavior is the same regardless.
    /// * `index` index of the control
    /// * `database` reference to the database
    ///
    /// returns:
    ///
    /// `CommandStatus` enumeration returning either `CommandStatus::Success` if the operation was accepted.
    fn operate(
        &mut self,
        control: T,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus;
}

/// Indices used by freeze operations
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FreezeIndices {
    /// All counters
    All,
    /// Range of counters (the range is inclusive)
    Range(u16, u16),
}

/// This object maps to the fields of g50v2
///
/// There is a table on page 57 of 1815-2012 that describes these 4 permutations
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FreezeInterval {
    /// Freeze once immediately
    FreezeOnceImmediately,
    /// Freeze once at the specified time
    FreezeOnceAtTime(Timestamp),
    /// Periodically freeze at intervals relative to the timestamp
    PeriodicallyFreeze(Timestamp, u32),
    /// Periodically freeze at intervals relative to the beginning of the current hour
    PeriodicallyFreezeRelative(u32),
}

impl FreezeInterval {
    /// construct a new FreezeTiming instance from the raw timestamp and interval fields
    pub fn new(timestamp: Timestamp, interval: u32) -> Self {
        match (timestamp.raw_value(), interval) {
            (0, 0) => Self::FreezeOnceImmediately,
            (_, 0) => Self::FreezeOnceAtTime(timestamp),
            (0, _) => Self::PeriodicallyFreezeRelative(interval),
            (_, _) => Self::PeriodicallyFreeze(timestamp, interval),
        }
    }

    /// decompose a FreezeTiming instance into the raw timestamp and interval fields
    pub fn get_time_and_interval(&self) -> (Timestamp, u32) {
        match self {
            FreezeInterval::FreezeOnceImmediately => (Timestamp::zero(), 0),
            FreezeInterval::FreezeOnceAtTime(t) => (*t, 0),
            FreezeInterval::PeriodicallyFreeze(t, i) => (*t, *i),
            FreezeInterval::PeriodicallyFreezeRelative(i) => (Timestamp::zero(), *i),
        }
    }
}

impl From<Group50Var2> for FreezeInterval {
    fn from(value: Group50Var2) -> Self {
        Self::new(value.time, value.interval)
    }
}

impl From<FreezeInterval> for Group50Var2 {
    fn from(value: FreezeInterval) -> Self {
        let (time, interval) = value.get_time_and_interval();
        Self { time, interval }
    }
}

/// Freeze operation type
#[cfg_attr(not(feature = "ffi"), non_exhaustive)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FreezeType {
    /// Copy the current value of a counter to the associated point
    ImmediateFreeze,
    /// Copy the current value of a counter to the associated point and
    /// clear the current value to 0
    FreezeAndClear,
    /// Freeze at a particular time
    FreezeAtTime(FreezeInterval),
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
    /// called before any controls are processed
    fn begin_fragment(&mut self) {}

    /// called after all controls have been processed
    ///
    /// The database handle may be used to process any changes accumulated in response
    /// to controls using a single lock/unlock cycle as opposed to doing it in every callback.
    ///
    /// note: This operation may be asynchronous if required
    fn end_fragment(&mut self, _database: &mut DatabaseHandle) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }
}

/// Struct with a default implementation of [ControlHandler](crate::outstation::ControlHandler)
/// that returns that same `CommandStatus` for every operation.
#[derive(Copy, Clone)]
pub struct DefaultControlHandler {
    status: CommandStatus,
}

impl DefaultControlHandler {
    /// create a boxed implementation of [ControlHandler](crate::outstation::ControlHandler) that
    /// returns [NotSupported](crate::app::control::CommandStatus::NotSupported) for every request.
    pub fn create() -> Box<dyn ControlHandler> {
        Self::with_status(CommandStatus::NotSupported)
    }

    /// create a boxed implementation of [ControlHandler](crate::outstation::ControlHandler) that
    /// returns the specified CommandStatus.
    pub fn with_status(status: CommandStatus) -> Box<dyn ControlHandler> {
        Box::new(DefaultControlHandler { status })
    }
}

impl ControlHandler for DefaultControlHandler {}

impl ControlSupport<Group12Var1> for DefaultControlHandler {
    fn select(
        &mut self,
        _control: Group12Var1,
        _index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group12Var1,
        _index: u16,
        _op_type: OperateType,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }
}

impl ControlSupport<Group41Var1> for DefaultControlHandler {
    fn select(
        &mut self,
        _control: Group41Var1,
        _index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group41Var1,
        _index: u16,
        _op_type: OperateType,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }
}

impl ControlSupport<Group41Var2> for DefaultControlHandler {
    fn select(
        &mut self,
        _control: Group41Var2,
        _index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group41Var2,
        _index: u16,
        _op_type: OperateType,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }
}

impl ControlSupport<Group41Var3> for DefaultControlHandler {
    fn select(
        &mut self,
        _control: Group41Var3,
        _index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group41Var3,
        _index: u16,
        _op_type: OperateType,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }
}

impl ControlSupport<Group41Var4> for DefaultControlHandler {
    fn select(
        &mut self,
        _control: Group41Var4,
        _index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.status
    }

    fn operate(
        &mut self,
        _control: Group41Var4,
        _index: u16,
        _op_type: OperateType,
        _database: &mut DatabaseHandle,
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
        database: &mut DatabaseHandle,
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

    fn select<I, F>(
        &mut self,
        seq: CountSequence<Prefix<I, T>>,
        database: &mut DatabaseHandle,
        mut func: F,
    ) where
        F: FnMut(T, I),
        I: Index,
    {
        for item in seq.iter() {
            let status = {
                if item.value.status() == CommandStatus::Success {
                    ControlSupport::<T>::select(
                        self,
                        item.value,
                        item.index.widen_to_u16(),
                        database,
                    )
                } else {
                    CommandStatus::FormatError
                }
            };
            func(item.value.with_status(status), item.index)
        }
    }
}
