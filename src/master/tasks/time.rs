use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::gen::variations::count::CountVariation;
use crate::app::gen::variations::fixed::{Group50Var1, Group50Var3};
use crate::app::parse::parser::{HeaderDetails, Response};
use crate::app::types::Timestamp;
use crate::master::association::Association;
use crate::master::error::{TaskError, TimeSyncError};
use crate::master::handle::Promise;
use crate::master::request::TimeSyncProcedure;
use crate::master::task::NonReadTask;
use crate::util::cursor::WriteError;
use std::convert::TryFrom;
use std::time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH};

enum State {
    MeasureDelay,
    WriteAbsoluteTime(Timestamp),
    RecordCurrentTime,
    WriteLastRecordedTime(Timestamp),
}

pub(crate) struct TimeSyncTask {
    is_auto_task: bool,
    state: State,
    promise: Promise<Result<(), TimeSyncError>>,
}

impl TimeSyncProcedure {
    fn get_start_state(&self) -> State {
        match self {
            TimeSyncProcedure::LAN => State::RecordCurrentTime,
            TimeSyncProcedure::NonLAN => State::MeasureDelay,
        }
    }
}

impl TimeSyncTask {
    fn new(is_auto_task: bool, state: State, promise: Promise<Result<(), TimeSyncError>>) -> Self {
        Self {
            is_auto_task,
            state,
            promise,
        }
    }

    fn change_state(self, state: State) -> Self {
        TimeSyncTask::new(self.is_auto_task, state, self.promise)
    }

    pub(crate) fn get_procedure(
        procedure: TimeSyncProcedure,
        is_auto_task: bool,
        promise: Promise<Result<(), TimeSyncError>>,
    ) -> Self {
        Self::new(is_auto_task, procedure.get_start_state(), promise)
    }

    pub(crate) fn wrap(self) -> NonReadTask {
        NonReadTask::TimeSync(self)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::MeasureDelay => FunctionCode::DelayMeasure,
            State::WriteAbsoluteTime(_) => FunctionCode::Write,
            State::RecordCurrentTime => FunctionCode::RecordCurrentTime,
            State::WriteLastRecordedTime(_) => FunctionCode::Write,
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self.state {
            State::MeasureDelay => Ok(()),
            State::WriteAbsoluteTime(x) => writer.write_count_of_one(Group50Var1 { time: x }),
            State::RecordCurrentTime => Ok(()),
            State::WriteLastRecordedTime(x) => writer.write_count_of_one(Group50Var3 { time: x }),
        }
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(
        self,
        association: &mut Association,
        request_tx: SystemTime,
        response: Response,
    ) -> Option<NonReadTask> {
        match self.state {
            State::MeasureDelay => self.handle_delay_measure(association, request_tx, response),
            State::WriteAbsoluteTime(_) => self.handle_write_absolute_time(association, response),
            State::RecordCurrentTime => self.handle_record_current_time(request_tx, response),
            State::WriteLastRecordedTime(_) => {
                self.handle_write_last_recorded_time(association, response)
            }
        }
    }

    fn handle_delay_measure(
        self,
        association: &mut Association,
        request_tx: SystemTime,
        response: Response,
    ) -> Option<NonReadTask> {
        let now = SystemTime::now();

        let interval = match now.duration_since(request_tx) {
            Ok(x) => x,
            Err(_) => {
                log::error!("clock rollback detected while synchronizing outstation");
                self.promise.complete(Err(TimeSyncError::ClockRollback));
                return None;
            }
        };

        let objects = match response.objects {
            Ok(x) => x,
            Err(err) => {
                self.promise
                    .complete(Err(TaskError::MalformedResponse(err).into()));
                return None;
            }
        };

        let delay_ms: Option<u16> = objects.get_only_header().and_then(|x| match x.details {
            HeaderDetails::OneByteCount(1, CountVariation::Group52Var2(seq)) => {
                match seq.single() {
                    Some(x) => Some(x.time),
                    None => None,
                }
            }
            _ => None,
        });

        let delay_ms = match delay_ms {
            Some(x) => x,
            None => {
                log::warn!("received unexpected header(s) in response to delay measure");
                self.promise
                    .complete(Err(TaskError::UnexpectedResponseHeaders.into()));
                return None;
            }
        };

        // IEEE 1815-2012, pg 301:(Time at [D] – Time at [A] – outstation processing delay) / 2.
        let propagation_delay: Duration =
            match interval.checked_sub(Duration::from_millis(delay_ms as u64)) {
                Some(x) => (x / 2),
                None => {
                    log::warn!("outstation time delay is larger than the response delay");
                    self.promise
                        .complete(Err(TimeSyncError::BadOutstationTimeDelay(delay_ms)));
                    return None;
                }
            };

        let time = match association.get_system_time().duration_since(UNIX_EPOCH) {
            Err(err) => {
                log::error!("{}", err);
                self.promise.complete(Err(TimeSyncError::SystemTimeNotUnix));
                return None;
            }
            Ok(x) => x,
        };

        let timestamp = match Self::get_timestamp(time, propagation_delay) {
            Err(err) => {
                log::error!("{}", err);
                self.promise.complete(Err(err));
                return None;
            }
            Ok(x) => x,
        };

        Some(
            self.change_state(State::WriteAbsoluteTime(timestamp))
                .wrap(),
        )
    }

    fn handle_write_absolute_time(
        self,
        association: &mut Association,
        response: Response,
    ) -> Option<NonReadTask> {
        if self.is_auto_task {
            association.on_time_sync_iin_response(response.header.iin);
        }
        self.promise
            .complete(TimeSyncError::from_iin(response.header.iin));
        None
    }

    fn handle_record_current_time(
        self,
        request_tx: SystemTime,
        _response: Response,
    ) -> Option<NonReadTask> {
        let timestamp = match Self::convert_to_timestamp(request_tx) {
            Err(err) => {
                self.promise.complete(Err(err));
                return None;
            }
            Ok(x) => x,
        };

        Some(
            self.change_state(State::WriteLastRecordedTime(timestamp))
                .wrap(),
        )
    }

    fn handle_write_last_recorded_time(
        self,
        association: &mut Association,
        response: Response,
    ) -> Option<NonReadTask> {
        if self.is_auto_task {
            association.on_time_sync_iin_response(response.header.iin);
        }
        self.promise
            .complete(TimeSyncError::from_iin(response.header.iin));
        None
    }

    fn convert_to_timestamp(time: SystemTime) -> Result<Timestamp, TimeSyncError> {
        Ok(Timestamp::new(u64::try_from(
            time.duration_since(UNIX_EPOCH)?.as_millis(),
        )?))
    }

    fn get_timestamp(
        now: Duration,
        propagation_delay: Duration,
    ) -> Result<Timestamp, TimeSyncError> {
        match now.checked_add(propagation_delay) {
            Some(x) => Ok(Timestamp::new(u64::try_from(x.as_millis())?)),
            None => Err(TimeSyncError::Overflow),
        }
    }
}

impl From<std::num::TryFromIntError> for TimeSyncError {
    fn from(_: std::num::TryFromIntError) -> Self {
        TimeSyncError::Overflow
    }
}

impl From<SystemTimeError> for TimeSyncError {
    fn from(_: SystemTimeError) -> Self {
        TimeSyncError::Overflow
    }
}
