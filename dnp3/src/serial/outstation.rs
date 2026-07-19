use crate::app::parse::options::ParseOptions;
use crate::app::{Listener, MaybeAsync, RetryStrategy};
use crate::link::reader::LinkModes;
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::serial::task::SerialTask;
use crate::serial::{PortState, SerialSettings};
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::{Enabled, Session};
use tracing::Instrument;

/// A fully configured serial outstation that has not yet opened its serial port.
#[cfg(feature = "unstable")]
pub struct SerialOutstation {
    task: OutstationTask,
}

/// A serial outstation task that owns an open and configured serial port.
#[cfg(feature = "unstable")]
#[must_use = "a SerialOutstationTask does nothing unless you call .run()"]
pub struct SerialOutstationTask {
    inner: OneShotSerialOutstationTask,
}

struct OneShotSerialOutstationTask {
    path: String,
    serial: tokio_serial::SerialStream,
    task: OutstationTask,
}

#[cfg(feature = "unstable")]
impl SerialOutstation {
    /// Create a fully configured serial outstation without opening a serial port.
    pub fn new(
        config: OutstationConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, OutstationHandle) {
        let (task, handle) = create_outstation(config, application, information, control_handler);
        (Self { task }, handle)
    }

    /// Open and configure the serial port, producing a task that is ready to run.
    pub async fn open(
        self,
        path: &str,
        settings: SerialSettings,
    ) -> std::io::Result<SerialOutstationTask> {
        let serial = crate::serial::open(path, settings)?;
        Ok(SerialOutstationTask {
            inner: OneShotSerialOutstationTask::new(path, serial, self.task),
        })
    }
}

#[cfg(feature = "unstable")]
impl SerialOutstationTask {
    /// Run the outstation until it is shut down or the serial port fails.
    pub async fn run(self) {
        self.inner.run().await;
    }
}

impl OneShotSerialOutstationTask {
    fn new(path: &str, serial: tokio_serial::SerialStream, task: OutstationTask) -> Self {
        Self {
            path: path.to_owned(),
            serial,
            task,
        }
    }

    async fn run(mut self) {
        let mut io = PhysLayer::Serial(self.serial);
        let _ = self
            .task
            .run(&mut io)
            .instrument(tracing::info_span!("dnp3-outstation-serial", "port" = ?self.path))
            .await;
    }
}

fn create_outstation(
    config: OutstationConfig,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> (OutstationTask, OutstationHandle) {
    OutstationTask::create(
        Enabled::Yes,
        LinkModes::serial(),
        ParseOptions::get_static(),
        config,
        PhysAddr::None,
        application,
        information,
        control_handler,
    )
}

/// Spawn an outstation task onto the `Tokio` runtime. The task runs until the returned handle is dropped or
/// a serial port error occurs, e.g. a serial port is removed from the OS. It attempts to open
/// the serial port immediately, and fails if it cannot.
///
/// Most users should prefer [`spawn_outstation_serial_2`]. This function remains for API
/// compatibility reasons, but will likely be removed in future MAJOR release of the library.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_outstation_serial(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> std::io::Result<OutstationHandle> {
    let serial = crate::serial::open(path, settings)?;
    let (task, handle) = create_outstation(config, application, information, control_handler);
    tokio::spawn(OneShotSerialOutstationTask::new(path, serial, task).run());
    Ok(handle)
}

struct NullListener;

impl Listener<PortState> for NullListener {
    fn update(&mut self, _: PortState) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }
}

/// Spawns an outstation task onto the `Tokio` runtime. The task runs until the returned handle is dropped.
/// It is tolerant to the serial port being unavailable at startup or being removed from the OS. It
/// uses the provided `RetryStrategy` to determine when to retry the port if the port cannot be
/// opened or fails.
///
/// This function should be preferred over [`spawn_outstation_serial`] and will become the only method
/// available in a future 2.0 release.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
#[allow(clippy::too_many_arguments)]
pub fn spawn_outstation_serial_2(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    retry: RetryStrategy,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    listener: Box<dyn Listener<PortState>>,
) -> OutstationHandle {
    let (task, handle) = create_outstation(config, application, information, control_handler);

    let mut serial = SerialTask::new(path, settings, Session::outstation(task), retry, listener);

    let log_path = path.to_owned();
    let future = async move {
        serial
            .run()
            .instrument(tracing::info_span!("dnp3-outstation-serial", "port" = ?log_path))
            .await;
    };
    tokio::spawn(future);
    handle
}

#[cfg(all(test, feature = "unstable"))]
mod tests {
    use super::*;
    use crate::link::EndpointAddress;
    use crate::outstation::database::EventBufferConfig;

    struct NullApplication;
    impl OutstationApplication for NullApplication {}

    struct NullInformation;
    impl OutstationInformation for NullInformation {}

    fn create() -> (SerialOutstation, OutstationHandle) {
        let config = OutstationConfig::new(
            EndpointAddress::try_new(10).unwrap(),
            EndpointAddress::try_new(1).unwrap(),
            EventBufferConfig::all_types(0),
        );
        SerialOutstation::new(
            config,
            Box::new(NullApplication),
            Box::new(NullInformation),
            crate::outstation::DefaultControlHandler::create(),
        )
    }

    #[test]
    fn construction_does_not_require_a_runtime() {
        let (_outstation, _handle) = create();
    }

    #[tokio::test]
    async fn failed_open_does_not_produce_a_task() {
        let (outstation, _handle) = create();
        let result = outstation
            .open(
                "/path/that/does/not/exist/dnp3-test",
                SerialSettings::default(),
            )
            .await;
        assert!(result.is_err());
    }
}

/// This function was added post 1.0 to provide fault tolerance for outstation serial ports.
///
/// This function is implemented by calling [`spawn_outstation_serial_2`] with a post listener that does nothing.
pub fn spawn_outstation_serial_fault_tolerant(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    retry: RetryStrategy,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> OutstationHandle {
    spawn_outstation_serial_2(
        path,
        settings,
        config,
        retry,
        application,
        information,
        control_handler,
        Box::new(NullListener),
    )
}
