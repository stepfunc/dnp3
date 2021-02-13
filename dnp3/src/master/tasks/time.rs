use crate::app::enums::FunctionCode;
use crate::app::format::write::HeaderWriter;
use crate::app::gen::count::CountVariation;
use crate::app::parse::parser::Response;
use crate::app::types::Timestamp;
use crate::app::variations::{Group50Var1, Group50Var3};
use crate::master::association::Association;
use crate::master::error::{TaskError, TimeSyncError};
use crate::master::handle::Promise;
use crate::master::request::TimeSyncProcedure;
use crate::master::tasks::NonReadTask;
use crate::tokio::time::Instant;
use crate::util::cursor::WriteError;
use std::time::Duration;

enum State {
    MeasureDelay(Option<Instant>),
    WriteAbsoluteTime(Timestamp),
    RecordCurrentTime(Option<Timestamp>),
    WriteLastRecordedTime(Timestamp),
}

pub(crate) struct TimeSyncTask {
    state: State,
    promise: Promise<Result<(), TimeSyncError>>,
}

impl TimeSyncProcedure {
    fn get_start_state(&self) -> State {
        match self {
            TimeSyncProcedure::Lan => State::RecordCurrentTime(None),
            TimeSyncProcedure::NonLan => State::MeasureDelay(None),
        }
    }
}

impl TimeSyncTask {
    fn new(state: State, promise: Promise<Result<(), TimeSyncError>>) -> Self {
        Self { state, promise }
    }

    fn change_state(self, state: State) -> Self {
        TimeSyncTask::new(state, self.promise)
    }

    pub(crate) fn get_procedure(
        procedure: TimeSyncProcedure,
        promise: Promise<Result<(), TimeSyncError>>,
    ) -> Self {
        Self::new(procedure.get_start_state(), promise)
    }

    pub(crate) fn wrap(self) -> NonReadTask {
        NonReadTask::TimeSync(self)
    }

    pub(crate) fn start(mut self, association: &mut Association) -> Option<Self> {
        match &mut self.state {
            State::MeasureDelay(time) => {
                time.replace(Instant::now());

                match association.get_system_time() {
                    Some(_) => Some(self),
                    None => {
                        self.report_error(association, TimeSyncError::SystemTimeNotAvailable);
                        None
                    }
                }
            }
            State::WriteAbsoluteTime(_) => Some(self),
            State::RecordCurrentTime(time) => {
                *time = association.get_system_time();

                match time {
                    Some(_) => Some(self),
                    None => {
                        self.report_error(association, TimeSyncError::SystemTimeNotAvailable);
                        None
                    }
                }
            }
            State::WriteLastRecordedTime(_) => Some(self),
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::MeasureDelay(_) => FunctionCode::DelayMeasure,
            State::WriteAbsoluteTime(_) => FunctionCode::Write,
            State::RecordCurrentTime(_) => FunctionCode::RecordCurrentTime,
            State::WriteLastRecordedTime(_) => FunctionCode::Write,
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self.state {
            State::MeasureDelay(_) => Ok(()),
            State::WriteAbsoluteTime(x) => writer.write_count_of_one(Group50Var1 { time: x }),
            State::RecordCurrentTime(_) => Ok(()),
            State::WriteLastRecordedTime(x) => writer.write_count_of_one(Group50Var3 { time: x }),
        }
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        match self.promise {
            Promise::None => {
                if let Some(association) = association {
                    association.on_time_sync_failure(err.into());
                }
            }
            _ => self.promise.complete(Err(err.into())),
        }
    }

    pub(crate) fn handle(
        self,
        association: &mut Association,
        response: Response,
    ) -> Option<NonReadTask> {
        match self.state {
            State::MeasureDelay(time) => self.handle_delay_measure(association, time, response),
            State::WriteAbsoluteTime(_) => self.handle_write_absolute_time(association, response),
            State::RecordCurrentTime(time) => {
                self.handle_record_current_time(association, time, response)
            }
            State::WriteLastRecordedTime(_) => {
                self.handle_write_last_recorded_time(association, response)
            }
        }
    }

    fn handle_delay_measure(
        self,
        association: &mut Association,
        request_tx: Option<Instant>,
        response: Response,
    ) -> Option<NonReadTask> {
        let request_tx = request_tx.unwrap_or_else(Instant::now);
        let now = Instant::now();

        let interval = match now.checked_duration_since(request_tx) {
            Some(x) => x,
            None => {
                // This should NEVER happen. `crate::tokio::time::Instant` is guaranteed to be monotonic and nondecreasing.
                tracing::error!("clock rollback detected while synchronizing outstation");
                self.report_error(association, TimeSyncError::ClockRollback);
                return None;
            }
        };

        let objects = match response.objects {
            Ok(x) => x,
            Err(err) => {
                self.report_error(association, TaskError::MalformedResponse(err).into());
                return None;
            }
        };

        let delay_ms: Option<u16> = objects.get_only_header().and_then(|x| {
            if let Some(CountVariation::Group52Var2(seq)) = x.details.count() {
                match seq.single() {
                    Some(x) => Some(x.time),
                    None => None,
                }
            } else {
                None
            }
        });

        let delay_ms = match delay_ms {
            Some(x) => x,
            None => {
                tracing::warn!("received unexpected header(s) in response to delay measure");
                self.report_error(association, TaskError::UnexpectedResponseHeaders.into());
                return None;
            }
        };

        // IEEE 1815-2012, pg 301:(Time at [D] – Time at [A] – outstation processing delay) / 2.
        let propagation_delay: Duration =
            match interval.checked_sub(Duration::from_millis(delay_ms as u64)) {
                Some(x) => (x / 2),
                None => {
                    tracing::warn!("outstation time delay is larger than the response delay");
                    self.report_error(association, TimeSyncError::BadOutstationTimeDelay(delay_ms));
                    return None;
                }
            };

        let time = match association.get_system_time() {
            Some(time) => time,
            None => {
                tracing::warn!("system time not available");
                self.report_error(association, TimeSyncError::SystemTimeNotAvailable);
                return None;
            }
        };

        let timestamp = match Self::get_timestamp(time, propagation_delay) {
            Err(err) => {
                tracing::error!("{}", err);
                self.report_error(association, err);
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
        if !response.raw_objects.is_empty() {
            self.report_error(
                association,
                TimeSyncError::Task(TaskError::UnexpectedResponseHeaders),
            );
            return None;
        }

        if let Err(error) = TimeSyncError::from_iin(response.header.iin) {
            self.report_error(association, error);
        } else {
            self.report_success(association);
        }

        None
    }

    fn handle_record_current_time(
        self,
        association: &mut Association,
        recorded_time: Option<Timestamp>,
        response: Response,
    ) -> Option<NonReadTask> {
        if !response.raw_objects.is_empty() {
            self.report_error(
                association,
                TimeSyncError::Task(TaskError::UnexpectedResponseHeaders),
            );
            return None;
        }

        let recorded_time = recorded_time.expect("Recorded time should be set by the start method");
        Some(
            self.change_state(State::WriteLastRecordedTime(recorded_time))
                .wrap(),
        )
    }

    fn handle_write_last_recorded_time(
        self,
        association: &mut Association,
        response: Response,
    ) -> Option<NonReadTask> {
        if !response.raw_objects.is_empty() {
            self.report_error(
                association,
                TimeSyncError::Task(TaskError::UnexpectedResponseHeaders),
            );
            return None;
        }

        if let Err(error) = TimeSyncError::from_iin(response.header.iin) {
            self.report_error(association, error);
        } else {
            self.report_success(association);
        }

        None
    }

    fn get_timestamp(
        now: Timestamp,
        propagation_delay: Duration,
    ) -> Result<Timestamp, TimeSyncError> {
        match now.checked_add(propagation_delay) {
            Some(x) => Ok(x),
            None => Err(TimeSyncError::Overflow),
        }
    }

    fn report_success(self, association: &mut Association) {
        match self.promise {
            Promise::None => association.on_time_sync_success(),
            _ => self.promise.complete(Ok(())),
        }
    }

    fn report_error(self, association: &mut Association, error: TimeSyncError) {
        match self.promise {
            Promise::None => association.on_time_sync_failure(error),
            _ => self.promise.complete(Err(error)),
        }
    }
}

impl From<std::num::TryFromIntError> for TimeSyncError {
    fn from(_: std::num::TryFromIntError) -> Self {
        TimeSyncError::Overflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::format::write::*;
    use crate::app::header::*;
    use crate::app::parse::parser::ParsedFragment;
    use crate::app::parse::traits::{FixedSize, FixedSizeVariation};
    use crate::app::sequence::Sequence;
    use crate::master::handle::{AssociationHandler, NullHandler, ReadHandler};
    use crate::master::tasks::RequestWriter;
    use crate::util::cursor::WriteCursor;
    use std::cell::Cell;
    use std::time::SystemTime;

    fn response_control_field(seq: Sequence) -> Control {
        Control::response(seq, true, true, false)
    }

    struct SingleTimestampTestHandler {
        time: Cell<Option<Timestamp>>,
        handler: NullHandler,
    }

    impl SingleTimestampTestHandler {
        fn new(time: Timestamp) -> Self {
            Self {
                time: Cell::new(Some(time)),
                handler: NullHandler,
            }
        }
    }

    impl AssociationHandler for SingleTimestampTestHandler {
        fn get_system_time(&self) -> Option<Timestamp> {
            self.time.take()
        }

        fn get_integrity_handler(&mut self) -> &mut dyn ReadHandler {
            &mut self.handler
        }

        fn get_unsolicited_handler(&mut self) -> &mut dyn ReadHandler {
            &mut self.handler
        }

        fn get_default_poll_handler(&mut self) -> &mut dyn ReadHandler {
            &mut self.handler
        }
    }

    mod non_lan {

        use super::*;
        use crate::app::enums::QualifierCode;
        use crate::app::variations::Group52Var2;
        use crate::config::EndpointAddress;
        use crate::master::association::AssociationConfig;

        const OUTSTATION_DELAY_MS: u16 = 100;
        const TOTAL_DELAY_MS: u16 = 200;
        const PROPAGATION_DELAY_MS: u16 = (TOTAL_DELAY_MS - OUTSTATION_DELAY_MS) / 2;

        struct TestHandler {
            time: Timestamp,
            handler: NullHandler,
        }

        impl TestHandler {
            fn new(time: Timestamp) -> Self {
                Self {
                    time: time,
                    handler: NullHandler,
                }
            }
        }

        impl AssociationHandler for TestHandler {
            fn get_system_time(&self) -> Option<Timestamp> {
                Some(self.time)
            }

            fn get_integrity_handler(&mut self) -> &mut dyn ReadHandler {
                &mut self.handler
            }

            fn get_unsolicited_handler(&mut self) -> &mut dyn ReadHandler {
                &mut self.handler
            }

            fn get_default_poll_handler(&mut self) -> &mut dyn ReadHandler {
                &mut self.handler
            }
        }

        #[test]
        fn success() {
            run_nonlan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_measure_delay_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64));
                let task = send_measure_delay_response(task, &mut association).unwrap();
                let task = check_write_request(task, &mut association, system_time);
                send_write_response(task, &mut association);
                assert!(rx.try_recv().unwrap().is_ok());
            });
        }

        #[test]
        fn with_16bit_count() {
            run_nonlan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_measure_delay_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64));

                let task = {
                    let mut buffer = [0; 20];
                    let mut cursor = WriteCursor::new(&mut buffer);
                    let writer = start_response(
                        response_control_field(Sequence::default()),
                        ResponseFunction::Response,
                        Iin::default(),
                        &mut cursor,
                    )
                    .unwrap();
                    {
                        let cursor = writer.inner();
                        Group52Var2::VARIATION.write(cursor).unwrap();
                        QualifierCode::Count16.write(cursor).unwrap();
                        cursor.write_u16_le(1).unwrap();
                        Group52Var2 {
                            time: OUTSTATION_DELAY_MS,
                        }
                        .write(cursor)
                        .unwrap();
                    }
                    let response = ParsedFragment::parse(cursor.written())
                        .unwrap()
                        .to_response()
                        .unwrap();

                    task.handle(&mut association, response)
                }
                .unwrap();
                let task = check_write_request(task, &mut association, system_time);
                send_write_response(task, &mut association);
                assert!(rx.try_recv().unwrap().is_ok());
            });
        }

        #[test]
        fn delay_reported_by_outstation_greater_than_actual_delay() {
            run_nonlan_timesync_test(|task, _system_time, mut association, mut rx| {
                let task = check_measure_delay_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(1)); // This delay is less than the reported delay of the outstation
                assert!(send_measure_delay_response(task, &mut association).is_none());
                assert_eq!(
                    Err(TimeSyncError::BadOutstationTimeDelay(OUTSTATION_DELAY_MS)),
                    rx.try_recv().unwrap()
                );
            });
        }

        #[test]
        fn empty_measure_delay_response() {
            run_nonlan_timesync_test(|task, _system_time, mut association, mut rx| {
                let task = check_measure_delay_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64));
                {
                    let mut buffer = [0; 20];
                    let mut cursor = WriteCursor::new(&mut buffer);
                    let writer = start_response(
                        response_control_field(Sequence::default()),
                        ResponseFunction::Response,
                        Iin::default(),
                        &mut cursor,
                    )
                    .unwrap();

                    let response = writer.to_parsed().to_response().unwrap();

                    assert!(task.handle(&mut association, response).is_none());
                }
                assert_eq!(
                    rx.try_recv().unwrap(),
                    Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders))
                );
            });
        }

        #[test]
        fn non_empty_write_response() {
            run_nonlan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_measure_delay_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64));
                let task = send_measure_delay_response(task, &mut association).unwrap();
                let task = check_write_request(task, &mut association, system_time);
                {
                    let mut buffer = [0; 20];
                    let mut cursor = WriteCursor::new(&mut buffer);
                    let mut writer = start_response(
                        response_control_field(Sequence::default()),
                        ResponseFunction::Response,
                        Iin::default(),
                        &mut cursor,
                    )
                    .unwrap();
                    writer.write_class1230().unwrap();

                    let response = writer.to_parsed().to_response().unwrap();

                    assert!(task.handle(&mut association, response).is_none());
                }
                assert_matches!(
                    rx.try_recv().unwrap(),
                    Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders))
                );
            });
        }

        #[test]
        fn iin_bit_not_reset() {
            run_nonlan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_measure_delay_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64));
                let task = send_measure_delay_response(task, &mut association).unwrap();
                let task = check_write_request(task, &mut association, system_time);
                {
                    let mut buffer = [0; 20];
                    let mut cursor = WriteCursor::new(&mut buffer);
                    let writer = start_response(
                        response_control_field(Sequence::default()),
                        ResponseFunction::Response,
                        Iin::new(Iin1::new(0x10), Iin2::new(0x00)),
                        &mut cursor,
                    )
                    .unwrap();

                    let response = writer.to_parsed().to_response().unwrap();

                    assert!(task.handle(&mut association, response).is_none());
                }
                assert_matches!(rx.try_recv().unwrap(), Err(TimeSyncError::StillNeedsTime));
            });
        }

        #[test]
        fn error_response() {
            run_nonlan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_measure_delay_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64));
                let task = send_measure_delay_response(task, &mut association).unwrap();
                let task = check_write_request(task, &mut association, system_time);
                task.on_task_error(Some(&mut association), TaskError::ResponseTimeout);
                assert_eq!(
                    rx.try_recv().unwrap(),
                    Err(TimeSyncError::Task(TaskError::ResponseTimeout))
                );
            });
        }

        #[test]
        fn no_system_time_at_start() {
            run_nonlan_timesync_single_timestamp_test(
                |task, _system_time, mut association, mut rx| {
                    association.get_system_time(); // Empty the time

                    assert!(task.start(&mut association).is_none());
                    assert_eq!(
                        rx.try_recv().unwrap(),
                        Err(TimeSyncError::SystemTimeNotAvailable)
                    );
                },
            );
        }

        #[test]
        fn no_system_time_at_delay() {
            run_nonlan_timesync_single_timestamp_test(
                |task, _system_time, mut association, mut rx| {
                    let task = check_measure_delay_request(task, &mut association);
                    crate::tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64));
                    assert!(send_measure_delay_response(task, &mut association).is_none());
                    assert_eq!(
                        rx.try_recv().unwrap(),
                        Err(TimeSyncError::SystemTimeNotAvailable)
                    );
                },
            );
        }

        fn run_nonlan_timesync_test(
            test: impl FnOnce(
                NonReadTask,
                Timestamp,
                Association,
                crate::tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
            ),
        ) {
            let system_time = Timestamp::try_from_system_time(SystemTime::now()).unwrap();
            let association = Association::new(
                EndpointAddress::from(1).unwrap(),
                AssociationConfig::default(),
                Box::new(TestHandler::new(system_time)),
            );
            let (tx, rx) = crate::tokio::sync::oneshot::channel();
            let task = NonReadTask::TimeSync(TimeSyncTask::get_procedure(
                TimeSyncProcedure::NonLan,
                Promise::OneShot(tx),
            ));

            test(task, system_time, association, rx);
        }

        fn run_nonlan_timesync_single_timestamp_test(
            test: impl FnOnce(
                NonReadTask,
                Timestamp,
                Association,
                crate::tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
            ),
        ) {
            let system_time = Timestamp::try_from_system_time(SystemTime::now()).unwrap();
            let association = Association::new(
                EndpointAddress::from(1).unwrap(),
                AssociationConfig::default(),
                Box::new(SingleTimestampTestHandler::new(system_time)),
            );
            let (tx, rx) = crate::tokio::sync::oneshot::channel();
            let task = NonReadTask::TimeSync(TimeSyncTask::get_procedure(
                TimeSyncProcedure::NonLan,
                Promise::OneShot(tx),
            ));

            test(task, system_time, association, rx);
        }

        fn check_measure_delay_request(
            task: NonReadTask,
            association: &mut Association,
        ) -> NonReadTask {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let task = task.start(association).unwrap();
            let mut writer = start_request(
                Control::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::DelayMeasure);
            assert!(request.raw_objects.is_empty());

            task
        }

        fn send_measure_delay_response(
            task: NonReadTask,
            association: &mut Association,
        ) -> Option<NonReadTask> {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_response(
                response_control_field(Sequence::default()),
                ResponseFunction::Response,
                Iin::default(),
                &mut cursor,
            )
            .unwrap();
            writer
                .write_count_of_one(Group52Var2 {
                    time: OUTSTATION_DELAY_MS,
                })
                .unwrap();
            let response = writer.to_parsed().to_response().unwrap();

            task.handle(association, response)
        }

        fn check_write_request(
            task: NonReadTask,
            association: &mut Association,
            system_time: Timestamp,
        ) -> NonReadTask {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let task = task.start(association).unwrap();
            let mut writer = start_request(
                Control::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::Write);
            let header = request.objects.unwrap().get_only_header().unwrap();
            match header.details.count().unwrap() {
                CountVariation::Group50Var1(seq) => {
                    assert_eq!(
                        seq.single().unwrap().time,
                        system_time
                            .checked_add(Duration::from_millis(PROPAGATION_DELAY_MS as u64))
                            .unwrap()
                    );
                }
                _ => panic!("wrong WRITE content"),
            }

            task
        }

        fn send_write_response(task: NonReadTask, association: &mut Association) {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let writer = start_response(
                Control::request(Sequence::default()),
                ResponseFunction::Response,
                Iin::default(),
                &mut cursor,
            )
            .unwrap();
            let response = writer.to_parsed().to_response().unwrap();

            assert!(task.handle(association, response).is_none());
        }
    }

    mod lan {
        use super::*;
        use crate::config::EndpointAddress;
        use crate::master::association::AssociationConfig;

        const DELAY_MS: u16 = 200;

        #[test]
        fn success() {
            run_lan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_record_current_time_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(DELAY_MS as u64));
                let task = send_record_current_time_response(task, &mut association).unwrap();
                let task = check_write_request(task, &mut association, system_time);
                send_write_response(task, &mut association);
                assert!(rx.try_recv().unwrap().is_ok());
            });
        }

        #[test]
        fn non_empty_record_current_time_response() {
            run_lan_timesync_test(|task, _system_time, mut association, mut rx| {
                let task = check_record_current_time_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(DELAY_MS as u64));
                {
                    let mut buffer = [0; 20];
                    let mut cursor = WriteCursor::new(&mut buffer);
                    let mut writer = start_response(
                        response_control_field(Sequence::default()),
                        ResponseFunction::Response,
                        Iin::default(),
                        &mut cursor,
                    )
                    .unwrap();
                    writer.write_class1230().unwrap();

                    let response = writer.to_parsed().to_response().unwrap();

                    assert!(task.handle(&mut association, response).is_none());
                }
                assert_eq!(
                    rx.try_recv().unwrap(),
                    Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders))
                );
            });
        }

        #[test]
        fn non_empty_write_response() {
            run_lan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_record_current_time_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(DELAY_MS as u64));
                let task = send_record_current_time_response(task, &mut association).unwrap();
                let task = check_write_request(task, &mut association, system_time);
                {
                    let mut buffer = [0; 20];
                    let mut cursor = WriteCursor::new(&mut buffer);
                    let mut writer = start_response(
                        response_control_field(Sequence::default()),
                        ResponseFunction::Response,
                        Iin::default(),
                        &mut cursor,
                    )
                    .unwrap();
                    writer.write_class1230().unwrap();

                    let response = writer.to_parsed().to_response().unwrap();

                    assert!(task.handle(&mut association, response).is_none());
                }
                assert_eq!(
                    rx.try_recv().unwrap(),
                    Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders))
                );
            });
        }

        #[test]
        fn iin_bit_not_reset() {
            run_lan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_record_current_time_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(DELAY_MS as u64));
                let task = send_record_current_time_response(task, &mut association).unwrap();
                let task = check_write_request(task, &mut association, system_time);
                {
                    let mut buffer = [0; 20];
                    let mut cursor = WriteCursor::new(&mut buffer);
                    let writer = start_response(
                        response_control_field(Sequence::default()),
                        ResponseFunction::Response,
                        Iin::new(Iin1::new(0x10), Iin2::new(0x00)),
                        &mut cursor,
                    )
                    .unwrap();

                    let response = writer.to_parsed().to_response().unwrap();

                    assert!(task.handle(&mut association, response).is_none());
                }
                assert_matches!(rx.try_recv().unwrap(), Err(TimeSyncError::StillNeedsTime));
            });
        }

        #[test]
        fn error_response() {
            run_lan_timesync_test(|task, system_time, mut association, mut rx| {
                let task = check_record_current_time_request(task, &mut association);
                crate::tokio::time::advance(Duration::from_millis(DELAY_MS as u64));
                let task = send_record_current_time_response(task, &mut association).unwrap();
                let task = check_write_request(task, &mut association, system_time);
                task.on_task_error(Some(&mut association), TaskError::ResponseTimeout);
                assert_eq!(
                    rx.try_recv().unwrap(),
                    Err(TimeSyncError::Task(TaskError::ResponseTimeout))
                );
            });
        }

        #[test]
        fn no_system_time_available() {
            run_lan_timesync_test(|task, _system_time, mut association, mut rx| {
                association.get_system_time(); // Empty the time

                assert!(task.start(&mut association).is_none());
                assert_eq!(
                    rx.try_recv().unwrap(),
                    Err(TimeSyncError::SystemTimeNotAvailable)
                );
            });
        }

        fn run_lan_timesync_test(
            test: impl FnOnce(
                NonReadTask,
                Timestamp,
                Association,
                crate::tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
            ),
        ) {
            let system_time = Timestamp::try_from_system_time(SystemTime::now()).unwrap();
            let association = Association::new(
                EndpointAddress::from(1).unwrap(),
                AssociationConfig::default(),
                Box::new(SingleTimestampTestHandler::new(system_time)),
            );
            let (tx, rx) = crate::tokio::sync::oneshot::channel();
            let task = NonReadTask::TimeSync(TimeSyncTask::get_procedure(
                TimeSyncProcedure::Lan,
                Promise::OneShot(tx),
            ));

            test(task, system_time, association, rx);
        }

        fn check_record_current_time_request(
            task: NonReadTask,
            association: &mut Association,
        ) -> NonReadTask {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let task = task.start(association).unwrap();
            let mut writer = start_request(
                Control::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::RecordCurrentTime);
            assert!(request.raw_objects.is_empty());
            assert!(association.get_system_time().is_none());
            task
        }

        fn send_record_current_time_response(
            task: NonReadTask,
            association: &mut Association,
        ) -> Option<NonReadTask> {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let writer = start_response(
                response_control_field(Sequence::default()),
                ResponseFunction::Response,
                Iin::default(),
                &mut cursor,
            )
            .unwrap();
            let response = writer.to_parsed().to_response().unwrap();

            task.handle(association, response)
        }

        fn check_write_request(
            task: NonReadTask,
            association: &mut Association,
            system_time: Timestamp,
        ) -> NonReadTask {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let task = task.start(association).unwrap();
            let mut writer = start_request(
                Control::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::Write);
            let header = request.objects.unwrap().get_only_header().unwrap();
            match header.details.count().unwrap() {
                CountVariation::Group50Var3(seq) => {
                    assert_eq!(seq.single().unwrap().time, system_time);
                }
                _ => panic!("wrong WRITE content"),
            }

            task
        }

        fn send_write_response(task: NonReadTask, association: &mut Association) {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let writer = start_response(
                Control::request(Sequence::default()),
                ResponseFunction::Response,
                Iin::default(),
                &mut cursor,
            )
            .unwrap();
            let response = writer.to_parsed().to_response().unwrap();

            assert!(task.handle(association, response).is_none());
        }
    }
}
