//! The open, retry, disable and shutdown sequence of a serial master and outstation
//!
//! Opening a port that doesn't exist works anywhere; the tests of a session open a pseudo-terminal
//! and only run on Linux, where reading a pseudo-terminal whose other side closed fails with EIO.
//! With the clock paused, a pending timer makes the clock jump ahead while a
//! task waits on the port, so the outstation's session timers can move the end of its session.

use std::time::Duration;

use crate::app::RetryStrategy;
use crate::link::EndpointAddress;
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::outstation::database::EventBufferConfig;
use crate::outstation::{
    DefaultControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::serial::{PortState, SerialSettings};
use crate::util::test_support::{watchdog, Timeline};

const MISSING_PORT: &str = "/path/that/does/not/exist/dnp3-test";

const fn secs(x: u64) -> Duration {
    Duration::from_secs(x)
}

struct NullApplication;
impl OutstationApplication for NullApplication {}

struct NullInformation;
impl OutstationInformation for NullInformation {}

fn spawn_outstation(path: &str, retry: RetryStrategy) -> (OutstationHandle, Timeline<PortState>) {
    let (listener, timeline) = Timeline::new();
    let config = OutstationConfig::new(
        EndpointAddress::try_new(1024).unwrap(),
        EndpointAddress::try_new(1).unwrap(),
        EventBufferConfig::all_types(0),
    );
    let handle = crate::serial::spawn_outstation_serial_2(
        path,
        SerialSettings::default(),
        config,
        retry,
        Box::new(NullApplication),
        Box::new(NullInformation),
        DefaultControlHandler::create(),
        listener,
    );
    (handle, timeline)
}

fn spawn_master(path: &str, retry_delay: Duration) -> (MasterChannel, Timeline<PortState>) {
    let (listener, timeline) = Timeline::new();
    let channel = crate::serial::spawn_master_serial(
        MasterChannelConfig::new(EndpointAddress::try_new(1).unwrap()),
        path,
        SerialSettings::default(),
        retry_delay,
        listener,
    );
    (channel, timeline)
}

#[tokio::test(start_paused = true)]
async fn outstation_retries_failed_opens_with_growing_delays_kept_across_a_disable() {
    watchdog(async {
        let (mut handle, mut timeline) =
            spawn_outstation(MISSING_PORT, RetryStrategy::new(secs(1), secs(4)));
        timeline.expect(secs(0), PortState::Wait(secs(1))).await;
        timeline.expect(secs(1), PortState::Wait(secs(2))).await;
        timeline.expect(secs(3), PortState::Wait(secs(4))).await;
        timeline.expect(secs(7), PortState::Wait(secs(4))).await;
        handle.disable().await.unwrap();
        timeline.expect(secs(7), PortState::Disabled).await;

        tokio::time::sleep(secs(10)).await;
        timeline.assert_quiet();
        handle.enable().await.unwrap();
        timeline.expect(secs(17), PortState::Wait(secs(4))).await;
        drop(handle);
        timeline.expect(secs(17), PortState::Shutdown).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn master_starts_disabled_and_retries_failed_opens_with_a_fixed_delay() {
    watchdog(async {
        let (mut channel, mut timeline) = spawn_master(MISSING_PORT, secs(2));
        timeline.expect(secs(0), PortState::Disabled).await;
        tokio::time::sleep(secs(5)).await;
        timeline.assert_quiet();
        channel.enable().await.unwrap();
        timeline.expect(secs(5), PortState::Wait(secs(2))).await;
        timeline.expect(secs(7), PortState::Wait(secs(2))).await;
        timeline.expect(secs(9), PortState::Wait(secs(2))).await;
        drop(channel);
        timeline.expect(secs(9), PortState::Shutdown).await;
    })
    .await;
}

#[cfg(target_os = "linux")]
mod pty {
    use super::*;
    use std::path::PathBuf;
    use tokio::io::AsyncReadExt;
    use tokio_serial::{SerialPort, SerialStream};

    /// A path in the temporary directory that is removed when dropped
    struct Link(PathBuf);

    impl Link {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("dnp3-{}-{}", std::process::id(), name));
            drop(std::fs::remove_file(&path));
            Self(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }

        /// Point the link at a new pseudo-terminal, returning the controlling side
        fn connect(&self) -> SerialStream {
            let (controller, port) = SerialStream::pair().unwrap();
            std::os::unix::fs::symlink(port.name().unwrap(), &self.0).unwrap();
            controller
        }

        fn remove(&self) {
            std::fs::remove_file(&self.0).unwrap();
        }
    }

    impl Drop for Link {
        fn drop(&mut self) {
            drop(std::fs::remove_file(&self.0));
        }
    }

    /// The port side was closed, so reading the controlling side fails
    async fn assert_closed(controller: &mut SerialStream) {
        let mut buffer = [0; 16];
        assert_eq!(
            controller
                .read(&mut buffer)
                .await
                .map_err(|err| err.raw_os_error()),
            Err(Some(5)) // EIO
        );
    }

    #[tokio::test(start_paused = true)]
    async fn outstation_session_resets_the_back_off() {
        watchdog(async {
            let link = Link::new("outstation-back-off");
            let (handle, mut timeline) =
                spawn_outstation(link.path(), RetryStrategy::new(secs(1), secs(4)));
            timeline.expect(secs(0), PortState::Wait(secs(1))).await;
            timeline.expect(secs(1), PortState::Wait(secs(2))).await;
            timeline.expect(secs(3), PortState::Wait(secs(4))).await;

            let controller = link.connect();
            timeline.expect(secs(7), PortState::Open).await;
            // end the session, and remove the link before another pseudo-terminal can reuse its path
            drop(controller);
            link.remove();
            let at = timeline.expect_state(PortState::Wait(secs(1))).await;
            timeline
                .expect(at.saturating_add(secs(1)), PortState::Wait(secs(2)))
                .await;
            drop(handle);
            timeline
                .expect(at.saturating_add(secs(1)), PortState::Shutdown)
                .await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn master_session_ends_with_a_retry_wait() {
        watchdog(async {
            let link = Link::new("master-retry");
            let mut controller = link.connect();
            let (mut channel, mut timeline) = spawn_master(link.path(), secs(2));
            timeline.expect(secs(0), PortState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), PortState::Open).await;

            drop(controller);
            link.remove();
            timeline.expect(secs(0), PortState::Wait(secs(2))).await;
            controller = link.connect();
            timeline.expect(secs(2), PortState::Open).await;
            drop(channel);
            timeline.expect(secs(2), PortState::Shutdown).await;
            assert_closed(&mut controller).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn master_disable_during_a_session_closes_the_port_until_enabled() {
        watchdog(async {
            let link = Link::new("master-disable");
            let mut controller = link.connect();
            let (mut channel, mut timeline) = spawn_master(link.path(), secs(2));
            timeline.expect(secs(0), PortState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), PortState::Open).await;
            channel.disable().await.unwrap();
            timeline.expect(secs(0), PortState::Disabled).await;
            assert_closed(&mut controller).await;

            tokio::time::sleep(secs(10)).await;
            timeline.assert_quiet();
            channel.enable().await.unwrap();
            timeline.expect(secs(10), PortState::Open).await;
            drop(channel);
            timeline.expect(secs(10), PortState::Shutdown).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn master_disable_during_a_retry_wait_stops_at_once() {
        watchdog(async {
            let link = Link::new("master-disable-wait");
            let controller = link.connect();
            let (mut channel, mut timeline) = spawn_master(link.path(), secs(5));
            timeline.expect(secs(0), PortState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), PortState::Open).await;
            drop(controller);
            link.remove();
            timeline.expect(secs(0), PortState::Wait(secs(5))).await;
            channel.disable().await.unwrap();
            timeline.expect(secs(0), PortState::Disabled).await;
            drop(channel);
            timeline.expect(secs(0), PortState::Shutdown).await;
        })
        .await;
    }
}
