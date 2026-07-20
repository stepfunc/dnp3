/// A fully configured DNP3 outstation that is ready to run.
///
/// This task represents one outstation endpoint. It is the caller's responsibility to run it on
/// a Tokio runtime. No tracing span is attached automatically, so the caller may instrument the
/// [`run`](Self::run) future as desired.
#[must_use = "an OutstationTask does nothing unless you call .run()"]
pub struct OutstationTask {
    inner: OutstationTaskType,
}

enum OutstationTaskType {
    #[cfg(feature = "serial")]
    Serial(crate::serial::outstation::OneShotSerialOutstationTask),
    Tcp(crate::tcp::server_task::ServerTask),
}

impl OutstationTask {
    #[cfg(feature = "serial")]
    pub(crate) fn serial(task: crate::serial::outstation::OneShotSerialOutstationTask) -> Self {
        Self {
            inner: OutstationTaskType::Serial(task),
        }
    }

    pub(crate) fn tcp(task: crate::tcp::server_task::ServerTask) -> Self {
        Self {
            inner: OutstationTaskType::Tcp(task),
        }
    }

    /// Run the outstation until it is shut down or the transport fails.
    pub async fn run(self) {
        match self.inner {
            #[cfg(feature = "serial")]
            OutstationTaskType::Serial(task) => task.run().await,
            OutstationTaskType::Tcp(mut task) => {
                let _ = task.run().await;
            }
        }
    }
}
