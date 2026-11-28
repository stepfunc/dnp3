// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::Duration;

use crate::app::format::write::HeaderWriter;
use crate::app::gen::count::CountVariation;
use crate::app::parse::parser::Response;
use crate::app::variations::{Group50Var1, Group50Var3};
use crate::app::FunctionCode;
use crate::app::Timestamp;
use crate::master::association::{Association, MasterClock};
use crate::master::error::{TaskErrorKind, TimeSyncError, TimeSyncErrorKind};
use crate::master::promise::Promise;
use crate::master::request::TimeSyncProcedure;
use crate::master::tasks::empty_response::expect_empty;
use crate::master::tasks::{NonReadTask, RequestError, SolicitedTask, Step, Task};
use crate::master::TaskType;

use crate::util::clock::MonotonicTime;

/// Who receives the outcome of a time synchronization
enum Requester {
    /// Requested through the API
    User(Promise<Result<(), TimeSyncError>>),
    /// Started automatically, reported to the association
    Auto,
}

impl Requester {
    /// Fail a time synchronization without an association. An automatic one has no one to tell
    fn fail(self, err: TaskErrorKind) {
        match self {
            Self::User(promise) => promise.complete(Err(TimeSyncErrorKind::from(err).into())),
            Self::Auto => {}
        }
    }
}

/// A time synchronization that hasn't started. Starting it captures the time the first
/// request is based on.
pub(crate) struct PendingTimeSync {
    procedure: TimeSyncProcedure,
    requester: Requester,
}

impl PendingTimeSync {
    /// A time synchronization requested through the API
    pub(crate) fn user(
        procedure: TimeSyncProcedure,
        promise: Promise<Result<(), TimeSyncError>>,
    ) -> Self {
        Self {
            procedure,
            requester: Requester::User(promise),
        }
    }

    /// A time synchronization started because the outstation needs the time
    pub(crate) fn auto(procedure: TimeSyncProcedure) -> Self {
        Self {
            procedure,
            requester: Requester::Auto,
        }
    }

    /// The function of the first request
    fn first_function(&self) -> FunctionCode {
        match self.procedure {
            TimeSyncProcedure::NonLan => FunctionCode::DelayMeasure,
            TimeSyncProcedure::Lan => FunctionCode::RecordCurrentTime,
            TimeSyncProcedure::DirectWriteAbsTime => FunctionCode::Write,
        }
    }

    /// Fail the time synchronization before it started, reporting to its requester
    pub(crate) fn on_task_error(self, err: TaskErrorKind) {
        self.requester.fail(err)
    }
}

enum State {
    /// time the delay measurement request is sent
    MeasureDelay(MonotonicTime),
    WriteAbsoluteTime(Timestamp),
    /// time the record current time request is sent
    RecordCurrentTime(Timestamp),
    WriteLastRecordedTime(Timestamp),
}

pub(crate) struct TimeSyncTask {
    state: State,
    requester: Requester,
}

impl From<TimeSyncTask> for Task {
    fn from(value: TimeSyncTask) -> Self {
        Task::Solicited(SolicitedTask::NonRead(NonReadTask::TimeSync(value)))
    }
}

/// Check the response to the WRITE that sets the outstation's time
fn check_write_response(response: &Response) -> Result<(), TimeSyncErrorKind> {
    expect_empty(response)?;
    if response.header.iin.iin1.get_need_time() {
        return Err(TimeSyncErrorKind::StillNeedsTime);
    }
    Ok(())
}

impl TimeSyncTask {
    fn new(state: State, requester: Requester) -> Self {
        Self { state, requester }
    }

    #[cfg(test)]
    pub(crate) fn wrap(self) -> NonReadTask {
        NonReadTask::TimeSync(self)
    }

    fn get_timestamp(
        now: Timestamp,
        propagation_delay: Duration,
    ) -> Result<Timestamp, TimeSyncErrorKind> {
        match now.checked_add(propagation_delay) {
            Some(x) => Ok(x),
            None => Err(TimeSyncErrorKind::Overflow),
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

    pub(crate) fn task_type(&self) -> TaskType {
        TaskType::TimeSync
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskErrorKind> {
        match self.state {
            State::MeasureDelay(_) => {}
            State::WriteAbsoluteTime(time) => writer.write_count_of_one(Group50Var1 { time })?,
            State::RecordCurrentTime(_) => {}
            State::WriteLastRecordedTime(x) => {
                writer.write_count_of_one(Group50Var3 { time: x })?
            }
        }
        Ok(())
    }

    /// Fail the time synchronization, reporting to its requester if it has one
    pub(crate) fn on_task_error(self, err: TaskErrorKind) {
        self.requester.fail(err)
    }
}

impl<C: MasterClock> Association<C> {
    /// Start a time synchronization, or report an error and return `None` if the system time
    /// isn't available
    pub(crate) fn start_time_sync(&mut self, task: PendingTimeSync) -> Option<TimeSyncTask> {
        let state = match task.procedure {
            TimeSyncProcedure::NonLan => {
                let request_tx = self.now();
                self.get_system_time()
                    .map(|_| State::MeasureDelay(request_tx))
            }
            TimeSyncProcedure::Lan => self.get_system_time().map(State::RecordCurrentTime),
            TimeSyncProcedure::DirectWriteAbsTime => {
                self.get_system_time().map(State::WriteAbsoluteTime)
            }
        };

        match state {
            Some(state) => Some(TimeSyncTask::new(state, task.requester)),
            None => {
                let err = TaskErrorKind::SystemTimeNotAvailable;
                if matches!(task.requester, Requester::Auto) {
                    let seq = self.seq();
                    self.notify_task_start(TaskType::TimeSync, task.first_function(), seq);
                    self.notify_task_fail(TaskType::TimeSync, err);
                }
                self.complete_time_sync_requester(task.requester, Err(err.into()));
                None
            }
        }
    }

    /// Handle the response to a time synchronization, returning it if it has another request
    pub(crate) fn respond_time_sync(
        &mut self,
        mut task: TimeSyncTask,
        response: Response<'_>,
    ) -> Result<Option<TimeSyncTask>, TaskErrorKind> {
        match self.time_sync_step(&mut task, response) {
            Ok(Step::Next) => Ok(Some(task)),
            Ok(Step::Done(())) => {
                self.complete_time_sync(task, Ok(()));
                Ok(None)
            }
            Err(err) => {
                self.complete_time_sync(task, Err(err));
                Err(err.task_error())
            }
        }
    }

    /// Report the outcome of a time synchronization that ran
    pub(crate) fn complete_time_sync(
        &mut self,
        task: TimeSyncTask,
        result: Result<(), TimeSyncErrorKind>,
    ) {
        self.complete_time_sync_requester(task.requester, result)
    }

    /// The time to write to the outstation, from the response to the delay measurement
    fn handle_delay_measure(
        &mut self,
        request_tx: MonotonicTime,
        response: Response,
    ) -> Result<Timestamp, TimeSyncErrorKind> {
        let now = self.now();

        let interval = match now.checked_duration_since(request_tx) {
            Some(x) => x,
            None => {
                // This should NEVER happen. The master's clock is monotonic
                tracing::error!("clock rollback detected while synchronizing outstation");
                return Err(TimeSyncErrorKind::ClockRollback);
            }
        };

        let header = response
            .get_only_object_header()
            .map_err(TaskErrorKind::from)?;

        let delay_ms: Option<u16> = if let Some(CountVariation::Group52Var2(seq)) = header.count() {
            seq.single().map(|x| x.time)
        } else {
            None
        };

        let delay_ms = match delay_ms {
            Some(x) => x,
            None => {
                tracing::warn!("received unexpected header(s) in response to delay measure");
                return Err(TaskErrorKind::UnexpectedResponseHeaders.into());
            }
        };

        // IEEE 1815-2012, pg 301:(Time at [D] – Time at [A] – outstation processing delay) / 2.
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "Duration division by the constant 2 cannot fail"
        )]
        let propagation_delay: Duration =
            match interval.checked_sub(Duration::from_millis(delay_ms.into())) {
                Some(x) => x / 2,
                None => {
                    tracing::warn!("outstation time delay is larger than the response delay");
                    return Err(TimeSyncErrorKind::BadOutstationTimeDelay(delay_ms));
                }
            };

        let time = match self.get_system_time() {
            Some(time) => time,
            None => {
                tracing::warn!("system time not available");
                return Err(TaskErrorKind::SystemTimeNotAvailable.into());
            }
        };

        TimeSyncTask::get_timestamp(time, propagation_delay)
            .inspect_err(|err| tracing::error!("{err}"))
    }

    /// Advance the time synchronization with the response to its last request
    fn time_sync_step(
        &mut self,
        task: &mut TimeSyncTask,
        response: Response<'_>,
    ) -> Result<Step<()>, TimeSyncErrorKind> {
        match task.state {
            State::MeasureDelay(request_tx) => {
                let timestamp = self.handle_delay_measure(request_tx, response)?;
                task.state = State::WriteAbsoluteTime(timestamp);
                Ok(Step::Next)
            }
            State::RecordCurrentTime(recorded_time) => {
                expect_empty(&response)?;
                task.state = State::WriteLastRecordedTime(recorded_time);
                Ok(Step::Next)
            }
            State::WriteAbsoluteTime(_) | State::WriteLastRecordedTime(_) => {
                check_write_response(&response)?;
                Ok(Step::Done(()))
            }
        }
    }

    /// Report the outcome to the user, or for an automatic time synchronization, to the schedule
    fn complete_time_sync_requester(
        &mut self,
        requester: Requester,
        result: Result<(), TimeSyncErrorKind>,
    ) {
        match requester {
            Requester::User(promise) => promise.complete(result.map_err(Into::into)),
            Requester::Auto => match result {
                Ok(()) => self.on_time_sync_success(),
                Err(err) => self.on_time_sync_failure(err),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime;

    use crate::app::format::write::*;
    use crate::app::parse::options::ParseOptions;
    use crate::app::parse::parser::ParsedFragment;
    use crate::app::Sequence;
    use crate::app::*;
    use crate::link::EndpointAddress;
    use crate::master::handler::AssociationHandler;
    use crate::master::tasks::RequestWriter;
    use crate::master::test_fixtures::association;
    use crate::master::AssociationConfig;
    use crate::util::clock::TokioClock;

    use scursor::WriteCursor;

    use super::*;

    /// g60v2, g60v3, g60v4 and g60v1 headers, which no time sync response has
    const CLASS_1230: &[u8] = &[60, 2, 0x06, 60, 3, 0x06, 60, 4, 0x06, 60, 1, 0x06];

    const NEED_TIME: Iin = Iin::new(Iin1::new(0x10), Iin2::new(0x00));

    /// Answer `task` with a response that has this IIN and these object headers
    async fn respond(
        task: NonReadTask,
        association: &mut Association<TokioClock>,
        iin: Iin,
        objects: &[u8],
    ) -> Result<Option<NonReadTask>, TaskErrorKind> {
        let mut buffer = [0; 20];
        let mut cursor = WriteCursor::new(&mut buffer);
        start_response(
            ControlField::response(Sequence::default(), true, true, false),
            ResponseFunction::Response,
            iin,
            &mut cursor,
        )
        .unwrap();
        cursor.write_bytes(objects).unwrap();
        let response = ParsedFragment::parse(ParseOptions::default(), cursor.written())
            .unwrap()
            .to_response()
            .unwrap();

        association.handle_non_read_response(task, response).await
    }

    struct SingleTimestampTestHandler {
        time: Arc<Mutex<Option<Timestamp>>>,
    }

    impl SingleTimestampTestHandler {
        fn time(&self) -> std::sync::MutexGuard<'_, Option<Timestamp>> {
            self.time.lock().unwrap()
        }

        fn new(time: Timestamp) -> Self {
            Self {
                time: Arc::new(Mutex::new(Some(time))),
            }
        }
    }

    struct TestHandler {
        time: Timestamp,
    }

    impl TestHandler {
        fn new(time: Timestamp) -> Self {
            Self { time }
        }
    }

    impl AssociationHandler for TestHandler {
        fn get_current_time(&self) -> Option<Timestamp> {
            Some(self.time)
        }
    }

    impl AssociationHandler for SingleTimestampTestHandler {
        fn get_current_time(&self) -> Option<Timestamp> {
            self.time().take()
        }
    }

    fn time_sync_setup(
        procedure: TimeSyncProcedure,
        handler: fn(Timestamp) -> Box<dyn AssociationHandler>,
    ) -> (
        PendingTimeSync,
        Timestamp,
        Association<TokioClock>,
        tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
    ) {
        let system_time = Timestamp::try_from_system_time(SystemTime::now()).unwrap();
        let association = association(
            EndpointAddress::try_new(1).unwrap(),
            AssociationConfig::default(),
            handler(system_time),
            TokioClock::new(),
        );
        let (promise, rx) = Promise::one_shot();
        let task = PendingTimeSync::user(procedure, promise);

        (task, system_time, association, rx)
    }

    async fn send_write_response(task: NonReadTask, association: &mut Association<TokioClock>) {
        respond(task, association, Iin::default(), &[])
            .await
            .unwrap();
    }

    mod non_lan {
        use super::*;

        const OUTSTATION_DELAY_MS: u16 = 100;
        const TOTAL_DELAY_MS: u16 = 200;
        const PROPAGATION_DELAY_MS: u16 = (TOTAL_DELAY_MS - OUTSTATION_DELAY_MS) / 2;

        fn non_lan_time_sync_setup() -> (
            PendingTimeSync,
            Timestamp,
            Association<TokioClock>,
            tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
        ) {
            time_sync_setup(TimeSyncProcedure::NonLan, |time| {
                Box::new(TestHandler::new(time))
            })
        }

        fn non_lan_time_sync_single_time_setup() -> (
            PendingTimeSync,
            Timestamp,
            Association<TokioClock>,
            tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
        ) {
            time_sync_setup(TimeSyncProcedure::NonLan, |time| {
                Box::new(SingleTimestampTestHandler::new(time))
            })
        }

        #[tokio::test(start_paused = true)]
        async fn success() {
            let (task, system_time, mut association, mut rx) = non_lan_time_sync_setup();
            let task = check_measure_delay_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS.into())).await;
            let task = send_measure_delay_response(task, &mut association)
                .await
                .unwrap()
                .unwrap();
            let task = check_write_request(task, system_time);
            send_write_response(task, &mut association).await;
            rx.try_recv().unwrap().unwrap();
        }

        #[tokio::test(start_paused = true)]
        async fn with_16bit_count() {
            let (task, system_time, mut association, mut rx) = non_lan_time_sync_setup();

            let task = check_measure_delay_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS.into())).await;

            // g52v2 with a 2-byte count of one
            let [low, high] = OUTSTATION_DELAY_MS.to_le_bytes();
            let task = respond(
                task,
                &mut association,
                Iin::default(),
                &[52, 2, 0x08, 0x01, 0x00, low, high],
            )
            .await
            .unwrap()
            .unwrap();

            let task = check_write_request(task, system_time);
            send_write_response(task, &mut association).await;
            rx.try_recv().unwrap().unwrap();
        }

        #[tokio::test(start_paused = true)]
        async fn delay_reported_by_outstation_greater_than_actual_delay() {
            let (task, _system_time, mut association, mut rx) = non_lan_time_sync_setup();
            let task = check_measure_delay_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(1)).await; // This delay is less than the reported delay of the outstation
            assert_eq!(
                send_measure_delay_response(task, &mut association)
                    .await
                    .err(),
                Some(TaskErrorKind::UnexpectedResponseHeaders)
            );
            assert_eq!(
                Err(TimeSyncErrorKind::BadOutstationTimeDelay(OUTSTATION_DELAY_MS).into()),
                rx.try_recv().unwrap()
            );
        }

        #[tokio::test(start_paused = true)]
        async fn empty_measure_delay_response() {
            let (task, _system_time, mut association, mut rx) = non_lan_time_sync_setup();
            let task = check_measure_delay_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS.into())).await;
            assert_eq!(
                respond(task, &mut association, Iin::default(), &[])
                    .await
                    .err(),
                Some(TaskErrorKind::UnexpectedResponseHeaders)
            );
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::UnexpectedResponseHeaders.into())
            );
        }

        #[tokio::test(start_paused = true)]
        async fn non_empty_write_response() {
            let (task, system_time, mut association, mut rx) = non_lan_time_sync_setup();
            let task = check_measure_delay_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS.into())).await;
            let task = send_measure_delay_response(task, &mut association)
                .await
                .unwrap()
                .unwrap();
            let task = check_write_request(task, system_time);
            assert_eq!(
                respond(task, &mut association, Iin::default(), CLASS_1230)
                    .await
                    .err(),
                Some(TaskErrorKind::UnexpectedResponseHeaders)
            );
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::UnexpectedResponseHeaders.into())
            );
        }

        #[tokio::test(start_paused = true)]
        async fn iin_bit_not_reset() {
            let (task, system_time, mut association, mut rx) = non_lan_time_sync_setup();

            let task = check_measure_delay_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS.into())).await;
            let task = send_measure_delay_response(task, &mut association)
                .await
                .unwrap()
                .unwrap();
            let task = check_write_request(task, system_time);
            assert_eq!(
                respond(task, &mut association, NEED_TIME, &[]).await.err(),
                Some(TaskErrorKind::UnexpectedResponseHeaders)
            );
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TimeSyncErrorKind::StillNeedsTime.into())
            );
        }

        #[tokio::test(start_paused = true)]
        async fn error_response() {
            let (task, system_time, mut association, mut rx) = non_lan_time_sync_setup();
            let task = check_measure_delay_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS.into())).await;
            let task = send_measure_delay_response(task, &mut association)
                .await
                .unwrap()
                .unwrap();
            let task = check_write_request(task, system_time);
            association.fail_non_read(task, TaskErrorKind::ResponseTimeout);
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::ResponseTimeout.into())
            );
        }

        #[test]
        fn no_system_time_at_start() {
            let (task, _system_time, mut association, mut rx) =
                non_lan_time_sync_single_time_setup();
            association.get_system_time(); // Empty the time

            assert!(association.start_time_sync(task).is_none());
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::SystemTimeNotAvailable.into())
            );
        }

        #[tokio::test(start_paused = true)]
        async fn no_system_time_at_delay() {
            let (task, _system_time, mut association, mut rx) =
                non_lan_time_sync_single_time_setup();
            let task = check_measure_delay_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(TOTAL_DELAY_MS.into())).await;
            assert_eq!(
                send_measure_delay_response(task, &mut association)
                    .await
                    .err(),
                Some(TaskErrorKind::SystemTimeNotAvailable)
            );
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::SystemTimeNotAvailable.into())
            );
        }

        fn check_measure_delay_request(
            task: PendingTimeSync,
            association: &mut Association<TokioClock>,
        ) -> NonReadTask {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let task = association.start_time_sync(task).unwrap().wrap();
            let mut writer = start_request(
                ControlField::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::DelayMeasure);
            //assert!(request.raw_objects.is_empty());

            task
        }

        async fn send_measure_delay_response(
            task: NonReadTask,
            association: &mut Association<TokioClock>,
        ) -> Result<Option<NonReadTask>, TaskErrorKind> {
            // g52v2 with a count of one
            let [low, high] = OUTSTATION_DELAY_MS.to_le_bytes();
            respond(
                task,
                association,
                Iin::default(),
                &[52, 2, 0x07, 0x01, low, high],
            )
            .await
        }

        fn check_write_request(task: NonReadTask, system_time: Timestamp) -> NonReadTask {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_request(
                ControlField::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::Write);
            let header = request.objects.unwrap().get_only_header().unwrap();
            match header.count().unwrap() {
                CountVariation::Group50Var1(seq) => {
                    assert_eq!(
                        seq.single().unwrap().time,
                        system_time
                            .checked_add(Duration::from_millis(PROPAGATION_DELAY_MS.into()))
                            .unwrap()
                    );
                }
                _ => panic!("wrong WRITE content"),
            }

            task
        }
    }

    mod lan {

        use super::*;

        const DELAY_MS: u16 = 200;

        #[tokio::test(start_paused = true)]
        async fn success() {
            let (task, system_time, mut association, mut rx) = lan_time_sync_setup();
            let task = check_record_current_time_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(DELAY_MS.into())).await;
            let task = send_record_current_time_response(task, &mut association)
                .await
                .unwrap()
                .unwrap();
            let task = check_write_request(task, system_time);
            send_write_response(task, &mut association).await;
            rx.try_recv().unwrap().unwrap();
        }

        #[tokio::test(start_paused = true)]
        async fn non_empty_write_response() {
            let (task, system_time, mut association, mut rx) = lan_time_sync_setup();
            let task = check_record_current_time_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(DELAY_MS.into())).await;
            let task = send_record_current_time_response(task, &mut association)
                .await
                .unwrap()
                .unwrap();
            let task = check_write_request(task, system_time);
            assert_eq!(
                respond(task, &mut association, Iin::default(), CLASS_1230)
                    .await
                    .err(),
                Some(TaskErrorKind::UnexpectedResponseHeaders)
            );
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::UnexpectedResponseHeaders.into())
            );
        }

        #[tokio::test(start_paused = true)]
        async fn error_response() {
            let (task, system_time, mut association, mut rx) = lan_time_sync_setup();
            let task = check_record_current_time_request(task, &mut association);
            tokio::time::advance(Duration::from_millis(DELAY_MS.into())).await;
            let task = send_record_current_time_response(task, &mut association)
                .await
                .unwrap()
                .unwrap();
            let task = check_write_request(task, system_time);
            association.fail_non_read(task, TaskErrorKind::ResponseTimeout);
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::ResponseTimeout.into())
            );
        }

        #[test]
        fn no_system_time_available() {
            let (task, _system_time, mut association, mut rx) = lan_time_sync_setup();
            association.get_system_time(); // Empty the time

            assert!(association.start_time_sync(task).is_none());
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::SystemTimeNotAvailable.into())
            );
        }

        fn lan_time_sync_setup() -> (
            PendingTimeSync,
            Timestamp,
            Association<TokioClock>,
            tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
        ) {
            time_sync_setup(TimeSyncProcedure::Lan, |time| {
                Box::new(SingleTimestampTestHandler::new(time))
            })
        }

        fn check_record_current_time_request(
            task: PendingTimeSync,
            association: &mut Association<TokioClock>,
        ) -> NonReadTask {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let task = association.start_time_sync(task).unwrap().wrap();
            let mut writer = start_request(
                ControlField::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::RecordCurrentTime);
            //assert!(request.raw_objects.is_empty());
            assert!(association.get_system_time().is_none());
            task
        }

        async fn send_record_current_time_response(
            task: NonReadTask,
            association: &mut Association<TokioClock>,
        ) -> Result<Option<NonReadTask>, TaskErrorKind> {
            respond(task, association, Iin::default(), &[]).await
        }

        fn check_write_request(task: NonReadTask, system_time: Timestamp) -> NonReadTask {
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_request(
                ControlField::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            assert_eq!(request.header.function, FunctionCode::Write);
            let header = request.objects.unwrap().get_only_header().unwrap();
            match header.count().unwrap() {
                CountVariation::Group50Var3(seq) => {
                    assert_eq!(seq.single().unwrap().time, system_time);
                }
                _ => panic!("wrong WRITE content"),
            }

            task
        }
    }

    mod direct_write_abs_time {
        use super::*;

        #[test]
        fn transitions_to_write_absolute_time() {
            let (task, system_time, mut association, _rx) = direct_write_setup();
            let task = association.start_time_sync(task).unwrap();

            // Should have transitioned to WriteAbsoluteTime
            assert_eq!(task.function(), FunctionCode::Write);

            // Verify it writes g50v1 with the system time
            let mut buffer = [0; 20];
            let mut cursor = WriteCursor::new(&mut buffer);
            let mut writer = start_request(
                ControlField::request(Sequence::default()),
                task.function(),
                &mut cursor,
            )
            .unwrap();
            task.write(&mut writer).unwrap();
            let request = writer.to_parsed().to_request().unwrap();

            let header = request.objects.unwrap().get_only_header().unwrap();
            match header.count().unwrap() {
                CountVariation::Group50Var1(seq) => {
                    assert_eq!(seq.single().unwrap().time, system_time);
                }
                _ => panic!("expected g50v1, not g50v3"),
            }
        }

        #[test]
        fn no_system_time_available() {
            let (task, _system_time, mut association, mut rx) = direct_write_setup();
            association.get_system_time(); // Empty the time

            assert!(association.start_time_sync(task).is_none());
            assert_eq!(
                rx.try_recv().unwrap(),
                Err(TaskErrorKind::SystemTimeNotAvailable.into())
            );
        }

        fn direct_write_setup() -> (
            PendingTimeSync,
            Timestamp,
            Association<TokioClock>,
            tokio::sync::oneshot::Receiver<Result<(), TimeSyncError>>,
        ) {
            time_sync_setup(TimeSyncProcedure::DirectWriteAbsTime, |time| {
                Box::new(SingleTimestampTestHandler::new(time))
            })
        }
    }
}
