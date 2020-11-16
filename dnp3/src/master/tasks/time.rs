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
use crate::util::cursor::WriteError;
use std::convert::TryFrom;
use std::time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH};
use tokio::time::Instant;

enum State {
    MeasureDelay,
    WriteAbsoluteTime(Timestamp),
    RecordCurrentTime(Option<SystemTime>),
    WriteLastRecordedTime(Timestamp),
}

pub(crate) struct TimeSyncTask {
    state: State,
    promise: Promise<Result<(), TimeSyncError>>,
}

impl TimeSyncProcedure {
    fn get_start_state(&self) -> State {
        match self {
            TimeSyncProcedure::LAN => State::RecordCurrentTime(None),
            TimeSyncProcedure::NonLAN => State::MeasureDelay,
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

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::MeasureDelay => FunctionCode::DelayMeasure,
            State::WriteAbsoluteTime(_) => FunctionCode::Write,
            State::RecordCurrentTime(_) => FunctionCode::RecordCurrentTime,
            State::WriteLastRecordedTime(_) => FunctionCode::Write,
        }
    }

    pub(crate) fn write(
        &mut self,
        writer: &mut HeaderWriter,
        association: &Association,
    ) -> Result<(), WriteError> {
        match &mut self.state {
            State::MeasureDelay => Ok(()),
            State::WriteAbsoluteTime(x) => writer.write_count_of_one(Group50Var1 { time: *x }),
            State::RecordCurrentTime(time) => {
                time.replace(association.get_system_time());
                Ok(())
            }
            State::WriteLastRecordedTime(x) => writer.write_count_of_one(Group50Var3 { time: *x }),
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
        request_tx: Instant,
        response: Response,
    ) -> Option<NonReadTask> {
        match self.state {
            State::MeasureDelay => self.handle_delay_measure(association, request_tx, response),
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
        request_tx: Instant,
        response: Response,
    ) -> Option<NonReadTask> {
        let now = Instant::now();

        let interval = match now.checked_duration_since(request_tx) {
            Some(x) => x,
            None => {
                // This should NEVER happen. `tokio::time::Instant` is guaranteed to be monotonic and nondecreasing.
                log::error!("clock rollback detected while synchronizing outstation");
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
                log::warn!("received unexpected header(s) in response to delay measure");
                self.report_error(association, TaskError::UnexpectedResponseHeaders.into());
                return None;
            }
        };

        // IEEE 1815-2012, pg 301:(Time at [D] – Time at [A] – outstation processing delay) / 2.
        let propagation_delay: Duration =
            match interval.checked_sub(Duration::from_millis(delay_ms as u64)) {
                Some(x) => (x / 2),
                None => {
                    log::warn!("outstation time delay is larger than the response delay");
                    self.report_error(association, TimeSyncError::BadOutstationTimeDelay(delay_ms));
                    return None;
                }
            };

        let time = match association.get_system_time().duration_since(UNIX_EPOCH) {
            Err(err) => {
                log::error!("{}", err);
                self.report_error(association, TimeSyncError::SystemTimeNotUnix);
                return None;
            }
            Ok(x) => x,
        };

        let timestamp = match Self::get_timestamp(time, propagation_delay) {
            Err(err) => {
                log::error!("{}", err);
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
        recorded_time: Option<SystemTime>,
        response: Response,
    ) -> Option<NonReadTask> {
        if !response.raw_objects.is_empty() {
            self.report_error(
                association,
                TimeSyncError::Task(TaskError::UnexpectedResponseHeaders),
            );
            return None;
        }

        let recorded_time = recorded_time.unwrap_or_else(|| association.get_system_time());
        match Self::convert_to_timestamp(recorded_time) {
            Ok(timestamp) => Some(
                self.change_state(State::WriteLastRecordedTime(timestamp))
                    .wrap(),
            ),
            Err(err) => {
                self.report_error(association, err);
                None
            }
        }
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

impl From<SystemTimeError> for TimeSyncError {
    fn from(_: SystemTimeError) -> Self {
        TimeSyncError::Overflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::format::write::*;
    use crate::app::header::*;
    use crate::app::parse::parser::{DecodeSettings, ParsedFragment};
    use crate::app::parse::traits::{FixedSize, FixedSizeVariation};
    use crate::app::sequence::Sequence;
    use crate::master::tasks::RequestWriter;
    use crate::prelude::master::*;
    use crate::util::cursor::WriteCursor;
    use std::cell::Cell;
    use std::future::Future;

    mod non_lan {
        use super::*;
        const OUTSTATION_DELAY_MS: u16 = 100;
        const TOTAL_DELAY_MS: u16 = 200;
        const PROPAGATION_DELAY_MS: u16 = (TOTAL_DELAY_MS - OUTSTATION_DELAY_MS) / 2;

        struct TestHandler {
            time: SystemTime,
            handler: NullHandler,
        }

        impl TestHandler {
            fn new(time: SystemTime) -> Self {
                Self {
                    time,
                    handler: NullHandler,
                }
            }
        }

        impl AssociationHandler for TestHandler {
            fn get_system_time(&self) -> SystemTime {
                self.time
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

        #[tokio::test]
        async fn success() {
            run_nonlan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_measure_delay_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64)).await;
                    let mut task =
                        send_measure_delay_response(task, request_tx, &mut association).unwrap();
                    check_write_request(&mut task, &association, system_time);
                    send_write_response(task, request_tx, &mut association);
                    assert!(rx.await.unwrap().is_ok());
                },
            )
            .await;
        }

        #[tokio::test]
        async fn with_16bit_count() {
            run_nonlan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_measure_delay_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64)).await;

                    let mut task = {
                        let mut buffer = [0; 20];
                        let mut cursor = WriteCursor::new(&mut buffer);
                        let writer = start_response(
                            Control::response(Sequence::default()),
                            ResponseFunction::Response,
                            IIN::default(),
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
                        let response =
                            ParsedFragment::parse(DecodeSettings::none(), cursor.written())
                                .unwrap()
                                .to_response()
                                .unwrap();

                        task.handle(request_tx, &mut association, response)
                    }
                    .unwrap();
                    check_write_request(&mut task, &association, system_time);
                    send_write_response(task, request_tx, &mut association);
                    assert!(rx.await.unwrap().is_ok());
                },
            )
            .await;
        }

        #[tokio::test]
        async fn delay_reported_by_outstation_greater_than_actual_delay() {
            run_nonlan_timesync_test(
                |mut task, _system_time, request_tx, mut association, rx| async move {
                    check_measure_delay_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(1)).await; // This delay is less than the reported delay of the outstation
                    assert!(
                        send_measure_delay_response(task, request_tx, &mut association).is_none()
                    );
                    assert_eq!(
                        Err(TimeSyncError::BadOutstationTimeDelay(OUTSTATION_DELAY_MS)),
                        rx.await.unwrap()
                    );
                },
            )
            .await;
        }

        #[tokio::test]
        async fn empty_measure_delay_response() {
            run_nonlan_timesync_test(
                |mut task, _system_time, request_tx, mut association, rx| async move {
                    check_measure_delay_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64)).await;
                    {
                        let mut buffer = [0; 20];
                        let mut cursor = WriteCursor::new(&mut buffer);
                        let writer = start_response(
                            Control::response(Sequence::default()),
                            ResponseFunction::Response,
                            IIN::default(),
                            &mut cursor,
                        )
                        .unwrap();

                        let response = writer.to_parsed().to_response().unwrap();

                        assert!(task
                            .handle(request_tx, &mut association, response)
                            .is_none());
                    }
                    assert_eq!(
                        rx.await.unwrap(),
                        Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders))
                    );
                },
            )
            .await;
        }

        #[tokio::test]
        async fn non_empty_write_response() {
            run_nonlan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_measure_delay_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64)).await;
                    let mut task =
                        send_measure_delay_response(task, request_tx, &mut association).unwrap();
                    check_write_request(&mut task, &association, system_time);
                    {
                        let mut buffer = [0; 20];
                        let mut cursor = WriteCursor::new(&mut buffer);
                        let mut writer = start_response(
                            Control::response(Sequence::default()),
                            ResponseFunction::Response,
                            IIN::default(),
                            &mut cursor,
                        )
                        .unwrap();
                        writer.write_class1230().unwrap();

                        let response = writer.to_parsed().to_response().unwrap();

                        assert!(task
                            .handle(request_tx, &mut association, response)
                            .is_none());
                    }
                    assert_matches!(
                        rx.await.unwrap(),
                        Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders))
                    );
                },
            )
            .await;
        }

        #[tokio::test]
        async fn iin_bit_not_reset() {
            run_nonlan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_measure_delay_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64)).await;
                    let mut task =
                        send_measure_delay_response(task, request_tx, &mut association).unwrap();
                    check_write_request(&mut task, &association, system_time);
                    {
                        let mut buffer = [0; 20];
                        let mut cursor = WriteCursor::new(&mut buffer);
                        let writer = start_response(
                            Control::response(Sequence::default()),
                            ResponseFunction::Response,
                            IIN::new(IIN1::new(0x10), IIN2::new(0x00)),
                            &mut cursor,
                        )
                        .unwrap();

                        let response = writer.to_parsed().to_response().unwrap();

                        assert!(task
                            .handle(request_tx, &mut association, response)
                            .is_none());
                    }
                    assert_matches!(rx.await.unwrap(), Err(TimeSyncError::StillNeedsTime));
                },
            )
            .await;
        }

        #[tokio::test]
        async fn error_response() {
            run_nonlan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_measure_delay_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS as u64)).await;
                    let mut task =
                        send_measure_delay_response(task, request_tx, &mut association).unwrap();
                    check_write_request(&mut task, &association, system_time);
                    task.on_task_error(Some(&mut association), TaskError::ResponseTimeout);
                    assert_eq!(
                        rx.await.unwrap(),
                        Err(TimeSyncError::Task(TaskError::ResponseTimeout))
                    );
                },
            )
            .await;
        }

        async fn run_nonlan_timesync_test<F: Future>(
            test: impl FnOnce(
                NonReadTask,
                SystemTime,
                Instant,
                Association,
                tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
            ) -> F,
        ) {
            tokio::time::pause();
            let system_time = SystemTime::now();
            let association = Association::new(
                1,
                Configuration::default(),
                Box::new(TestHandler::new(system_time)),
            );
            let (tx, rx) = tokio::sync::oneshot::channel();
            let task = NonReadTask::TimeSync(TimeSyncTask::get_procedure(
                TimeSyncProcedure::NonLAN,
                Promise::OneShot(tx),
            ));
            let request_tx = Instant::now();

            test(task, system_time, request_tx, association, rx).await;
        }

        fn check_measure_delay_request(task: &mut NonReadTask, association: &Association) {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_request(
                Control::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer, association).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::DelayMeasure);
            assert!(request.raw_objects.is_empty());
        }

        fn send_measure_delay_response(
            task: NonReadTask,
            request_tx: Instant,
            association: &mut Association,
        ) -> Option<NonReadTask> {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_response(
                Control::response(Sequence::default()),
                ResponseFunction::Response,
                IIN::default(),
                &mut cursor,
            )
            .unwrap();
            writer
                .write_count_of_one(Group52Var2 {
                    time: OUTSTATION_DELAY_MS,
                })
                .unwrap();
            let response = writer.to_parsed().to_response().unwrap();

            task.handle(request_tx, association, response)
        }

        fn check_write_request(
            task: &mut NonReadTask,
            association: &Association,
            system_time: SystemTime,
        ) {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_request(
                Control::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer, association).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::Write);
            let header = request.objects.unwrap().get_only_header().unwrap();
            match header.details.count().unwrap() {
                CountVariation::Group50Var1(seq) => {
                    assert_eq!(
                        seq.single().unwrap().time.raw_value(),
                        system_time
                            .checked_add(Duration::from_millis(PROPAGATION_DELAY_MS as u64))
                            .unwrap()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64
                    );
                }
                _ => panic!("wrong WRITE content"),
            }
        }

        fn send_write_response(
            task: NonReadTask,
            request_tx: Instant,
            association: &mut Association,
        ) {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let writer = start_response(
                Control::request(Sequence::default()),
                ResponseFunction::Response,
                IIN::default(),
                &mut cursor,
            )
            .unwrap();
            let response = writer.to_parsed().to_response().unwrap();

            assert!(task.handle(request_tx, association, response).is_none());
        }
    }

    mod lan {
        use super::*;

        const DELAY_MS: u16 = 200;

        struct TestHandler {
            time: SystemTime,
            handler: NullHandler,
            get_system_time_called: Cell<bool>,
        }

        impl TestHandler {
            fn new(time: SystemTime) -> Self {
                Self {
                    time,
                    handler: NullHandler,
                    get_system_time_called: Cell::new(false),
                }
            }
        }

        impl AssociationHandler for TestHandler {
            fn get_system_time(&self) -> SystemTime {
                if !self.get_system_time_called.get() {
                    self.get_system_time_called.set(true);
                    self.time
                } else {
                    SystemTime::UNIX_EPOCH
                }
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

        #[tokio::test]
        async fn success() {
            run_lan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_record_current_time_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(DELAY_MS as u64)).await;
                    let mut task =
                        send_record_current_time_response(task, request_tx, &mut association)
                            .unwrap();
                    check_write_request(&mut task, &association, system_time);
                    send_write_response(task, request_tx, &mut association);
                    assert!(rx.await.unwrap().is_ok());
                },
            )
            .await;
        }

        #[tokio::test]
        async fn non_empty_record_current_time_response() {
            run_lan_timesync_test(
                |mut task, _system_time, request_tx, mut association, rx| async move {
                    check_record_current_time_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(DELAY_MS as u64)).await;
                    {
                        let mut buffer = [0; 20];
                        let mut cursor = WriteCursor::new(&mut buffer);
                        let mut writer = start_response(
                            Control::response(Sequence::default()),
                            ResponseFunction::Response,
                            IIN::default(),
                            &mut cursor,
                        )
                        .unwrap();
                        writer.write_class1230().unwrap();

                        let response = writer.to_parsed().to_response().unwrap();

                        assert!(task
                            .handle(request_tx, &mut association, response)
                            .is_none());
                    }
                    assert_eq!(
                        rx.await.unwrap(),
                        Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders))
                    );
                },
            )
            .await;
        }

        #[tokio::test]
        async fn non_empty_write_response() {
            run_lan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_record_current_time_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(DELAY_MS as u64)).await;
                    let mut task =
                        send_record_current_time_response(task, request_tx, &mut association)
                            .unwrap();
                    check_write_request(&mut task, &association, system_time);
                    {
                        let mut buffer = [0; 20];
                        let mut cursor = WriteCursor::new(&mut buffer);
                        let mut writer = start_response(
                            Control::response(Sequence::default()),
                            ResponseFunction::Response,
                            IIN::default(),
                            &mut cursor,
                        )
                        .unwrap();
                        writer.write_class1230().unwrap();

                        let response = writer.to_parsed().to_response().unwrap();

                        assert!(task
                            .handle(request_tx, &mut association, response)
                            .is_none());
                    }
                    assert_eq!(
                        rx.await.unwrap(),
                        Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders))
                    );
                },
            )
            .await;
        }

        #[tokio::test]
        async fn iin_bit_not_reset() {
            run_lan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_record_current_time_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(DELAY_MS as u64)).await;
                    let mut task =
                        send_record_current_time_response(task, request_tx, &mut association)
                            .unwrap();
                    check_write_request(&mut task, &association, system_time);
                    {
                        let mut buffer = [0; 20];
                        let mut cursor = WriteCursor::new(&mut buffer);
                        let writer = start_response(
                            Control::response(Sequence::default()),
                            ResponseFunction::Response,
                            IIN::new(IIN1::new(0x10), IIN2::new(0x00)),
                            &mut cursor,
                        )
                        .unwrap();

                        let response = writer.to_parsed().to_response().unwrap();

                        assert!(task
                            .handle(request_tx, &mut association, response)
                            .is_none());
                    }
                    assert_matches!(rx.await.unwrap(), Err(TimeSyncError::StillNeedsTime));
                },
            )
            .await;
        }

        #[tokio::test]
        async fn error_response() {
            run_lan_timesync_test(
                |mut task, system_time, request_tx, mut association, rx| async move {
                    check_record_current_time_request(&mut task, &association);
                    tokio::time::advance(Duration::from_millis(DELAY_MS as u64)).await;
                    let mut task =
                        send_record_current_time_response(task, request_tx, &mut association)
                            .unwrap();
                    check_write_request(&mut task, &association, system_time);
                    task.on_task_error(Some(&mut association), TaskError::ResponseTimeout);
                    assert_eq!(
                        rx.await.unwrap(),
                        Err(TimeSyncError::Task(TaskError::ResponseTimeout))
                    );
                },
            )
            .await;
        }

        async fn run_lan_timesync_test<F: Future>(
            test: impl FnOnce(
                NonReadTask,
                SystemTime,
                Instant,
                Association,
                tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
            ) -> F,
        ) {
            tokio::time::pause();
            let system_time = SystemTime::now();
            let association = Association::new(
                1,
                Configuration::default(),
                Box::new(TestHandler::new(system_time)),
            );
            let (tx, rx) = tokio::sync::oneshot::channel();
            let task = NonReadTask::TimeSync(TimeSyncTask::get_procedure(
                TimeSyncProcedure::LAN,
                Promise::OneShot(tx),
            ));
            let request_tx = Instant::now();

            test(task, system_time, request_tx, association, rx).await;
        }

        fn check_record_current_time_request(task: &mut NonReadTask, association: &Association) {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_request(
                Control::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer, association).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::RecordCurrentTime);
            assert!(request.raw_objects.is_empty());
            assert_eq!(association.get_system_time(), SystemTime::UNIX_EPOCH);
        }

        fn send_record_current_time_response(
            task: NonReadTask,
            request_tx: Instant,
            association: &mut Association,
        ) -> Option<NonReadTask> {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let writer = start_response(
                Control::response(Sequence::default()),
                ResponseFunction::Response,
                IIN::default(),
                &mut cursor,
            )
            .unwrap();
            let response = writer.to_parsed().to_response().unwrap();

            task.handle(request_tx, association, response)
        }

        fn check_write_request(
            task: &mut NonReadTask,
            association: &Association,
            system_time: SystemTime,
        ) {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_request(
                Control::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer, association).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::Write);
            let header = request.objects.unwrap().get_only_header().unwrap();
            match header.details.count().unwrap() {
                CountVariation::Group50Var3(seq) => {
                    assert_eq!(
                        seq.single().unwrap().time.raw_value(),
                        system_time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
                    );
                }
                _ => panic!("wrong WRITE content"),
            }
        }

        fn send_write_response(
            task: NonReadTask,
            request_tx: Instant,
            association: &mut Association,
        ) {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let writer = start_response(
                Control::request(Sequence::default()),
                ResponseFunction::Response,
                IIN::default(),
                &mut cursor,
            )
            .unwrap();
            let response = writer.to_parsed().to_response().unwrap();

            assert!(task.handle(request_tx, association, response).is_none());
        }
    }
}
