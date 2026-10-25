// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::util::phys::{PhysAddr, PhysLayer};
use std::io::ErrorKind;
use std::net::SocketAddr;

enum UdpType {
    /// socket is only bound, not connected, communicates 1 to many
    Bound(tokio::net::UdpSocket),
    /// socket is bound AND connected, communicates 1 to 1
    Connected(tokio::net::UdpSocket),
    /// behaves like a bound socket, with datagrams and send errors supplied by a test
    #[cfg(test)]
    Scripted(scripted::ScriptedSocket),
}

enum UdpFactoryType {
    Bound {
        local: SocketAddr,
    },
    Connected {
        local: SocketAddr,
        remote: SocketAddr,
    },
}

pub(crate) struct UdpFactory(UdpFactoryType);

impl UdpFactory {
    pub(crate) fn bound(local: SocketAddr) -> Self {
        Self(UdpFactoryType::Bound { local })
    }

    pub(crate) fn connected(local: SocketAddr, remote: SocketAddr) -> Self {
        Self(UdpFactoryType::Connected { local, remote })
    }
}

impl UdpFactory {
    pub(crate) async fn open(&self) -> std::io::Result<PhysLayer> {
        match self.0 {
            UdpFactoryType::Bound { local } => {
                let layer = UdpLayer::bind(local).await?;
                Ok(PhysLayer::udp(layer))
            }
            UdpFactoryType::Connected { local, remote } => {
                let layer = UdpLayer::connect(local, remote).await?;
                Ok(PhysLayer::udp(layer))
            }
        }
    }
}

impl UdpLayer {
    async fn bind(local: SocketAddr) -> std::io::Result<Self> {
        let socket = tokio::net::UdpSocket::bind(local).await?;
        Ok(Self::bound(socket))
    }

    pub(crate) fn bound(socket: tokio::net::UdpSocket) -> Self {
        UdpLayer {
            inner: UdpType::Bound(socket),
        }
    }

    async fn connect(local: SocketAddr, remote: SocketAddr) -> std::io::Result<Self> {
        let socket = tokio::net::UdpSocket::bind(local).await?;
        socket.connect(remote).await?;
        Ok(UdpLayer {
            inner: UdpType::Connected(socket),
        })
    }
}

pub(crate) struct UdpLayer {
    inner: UdpType,
}

impl UdpType {
    pub(crate) async fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(usize, PhysAddr), std::io::Error> {
        match self {
            Self::Bound(x) => x
                .recv_from(buffer)
                .await
                .map(|(x, s)| (x, PhysAddr::Udp(s))),
            Self::Connected(x) => x.recv(buffer).await.map(|x| (x, PhysAddr::None)),
            #[cfg(test)]
            Self::Scripted(x) => x.read(buffer).await,
        }
    }

    pub(crate) async fn write(
        &mut self,
        data: &[u8],
        addr: PhysAddr,
    ) -> Result<usize, std::io::Error> {
        match self {
            UdpType::Bound(s) => s.send_to(data, destination(addr)?).await,
            UdpType::Connected(s) => s.send(data).await,
            #[cfg(test)]
            UdpType::Scripted(s) => s.write(data, addr),
        }
    }

    fn is_bound(&self) -> bool {
        match self {
            UdpType::Bound(_) => true,
            UdpType::Connected(_) => false,
            #[cfg(test)]
            UdpType::Scripted(s) => s.is_bound(),
        }
    }
}

fn destination(addr: PhysAddr) -> Result<SocketAddr, std::io::Error> {
    match addr {
        PhysAddr::None => Err(std::io::Error::other(
            "No destination specified for un-connected UDP socket",
        )),
        PhysAddr::Udp(x) => Ok(x),
    }
}

impl UdpLayer {
    pub(crate) async fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(usize, PhysAddr), std::io::Error> {
        let (count, source) = loop {
            match self.inner.read(buffer).await {
                // an empty datagram is a valid zero-length read, not the end of a stream
                Ok((0, _)) => tracing::debug!("Discarded empty UDP datagram"),
                // a bound socket can't send a reply to port 0
                Ok((_, PhysAddr::Udp(source))) if source.port() == 0 => {
                    tracing::debug!("Discarded UDP datagram from port 0 of {}", source.ip())
                }
                Ok(x) => break x,
                Err(err) if is_oversized_datagram(&err) => tracing::warn!(
                    "Discarded UDP datagram larger than the read buffer of {} bytes",
                    buffer.len()
                ),
                Err(err) => {
                    if is_icmp_error(&err) {
                        tracing::warn!("Trapped ICMP error on UDP read: {}", err);
                    } else {
                        return Err(err);
                    }
                }
            }
        };

        if count == buffer.len() {
            tracing::error!("UDP under-read w/ buffer size == {count}");
        }
        Ok((count, source))
    }

    pub(crate) async fn write_all(
        &mut self,
        data: &[u8],
        addr: PhysAddr,
    ) -> Result<(), std::io::Error> {
        let result = match self.inner.write(data, addr).await {
            // the error reports an earlier datagram, so this one wasn't sent yet
            Err(err) if is_icmp_error(&err) => {
                tracing::warn!("Trapped ICMP error on UDP write: {}", err);
                self.inner.write(data, addr).await
            }
            result => result,
        };
        match result {
            Ok(count) => {
                if count < data.len() {
                    tracing::error!("UDP under-write");
                }
            }
            // only the retry gets here, so this reports yet another earlier datagram
            Err(err) if is_icmp_error(&err) => {
                tracing::warn!("UDP datagram not sent after a second ICMP error: {}", err);
            }
            Err(err) if self.inner.is_bound() && is_destination_error(&err) => {
                tracing::warn!("UDP datagram to {:?} not sent: {}", addr, err);
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}

/// The Windows error for an earlier datagram whose time to live expired, which std leaves
/// uncategorized
const WSAENETRESET: i32 = 10052;

/// An ICMP error reports that an earlier datagram wasn't delivered. Windows reports port
/// unreachable as a reset and an expired time to live as `WSAENETRESET`, and Linux reports port
/// unreachable as a refusal on a connected socket.
fn is_icmp_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        ErrorKind::ConnectionReset | ErrorKind::ConnectionRefused
    ) || (cfg!(windows) && err.raw_os_error() == Some(WSAENETRESET))
}

/// The Windows error for a received datagram larger than the buffer, which it discards
const WSAEMSGSIZE: i32 = 10040;

/// A datagram too large for the read buffer. Windows fails the read without its length and
/// discards what didn't fit, while other platforms return the truncated length.
fn is_oversized_datagram(err: &std::io::Error) -> bool {
    cfg!(windows) && err.raw_os_error() == Some(WSAEMSGSIZE)
}

/// Linux also reports a send to port 0 as an invalid argument, which on other platforms means a
/// socket or programming error
const INVALID_INPUT_IS_DESTINATION: bool = cfg!(any(target_os = "linux", target_os = "android"));

/// A send error that a bound socket tolerates as the loss of one datagram, because the
/// destination can cause it: no route to it, a broadcast address, a firewall rule, or an address
/// that can't be sent to, such as port 0. It doesn't show that other destinations still work: a
/// firewall can deny every send, and on macOS an unavailable address also covers a bound address
/// that has gone away.
fn is_destination_error(err: &std::io::Error) -> bool {
    let kind = err.kind();
    matches!(
        kind,
        ErrorKind::AddrNotAvailable
            | ErrorKind::NetworkUnreachable
            | ErrorKind::HostUnreachable
            | ErrorKind::PermissionDenied
    ) || (INVALID_INPUT_IS_DESTINATION && kind == ErrorKind::InvalidInput)
}

#[cfg(test)]
mod scripted {
    use super::{destination, UdpLayer, UdpType};
    use crate::util::phys::PhysAddr;
    use std::collections::VecDeque;
    use std::io::ErrorKind;
    use std::net::SocketAddr;
    use tokio::sync::mpsc;

    pub(super) type Datagram = (Vec<u8>, SocketAddr);

    /// An error for a [`ScriptedSocket`] to return
    #[derive(Copy, Clone, Debug)]
    pub(super) enum Fault {
        Kind(ErrorKind),
        /// a platform error code
        Os(i32),
    }

    impl From<Fault> for std::io::Error {
        fn from(fault: Fault) -> Self {
            match fault {
                Fault::Kind(kind) => kind.into(),
                Fault::Os(code) => std::io::Error::from_raw_os_error(code),
            }
        }
    }

    /// A socket that receives the datagrams and errors a test supplies, and records the datagrams
    /// it sends
    ///
    /// Each send attempt first takes the next scripted error, if any are left. Without one, a
    /// send to port 0 fails with an invalid argument, as it does on Linux. A connected socket
    /// sends to its remote address, and reads datagrams without their source.
    pub(super) struct ScriptedSocket {
        received: mpsc::UnboundedReceiver<Result<Datagram, Fault>>,
        sent: mpsc::UnboundedSender<Datagram>,
        send_errors: VecDeque<Fault>,
        send_attempts: usize,
        connected: Option<SocketAddr>,
    }

    /// The test's side of a [`ScriptedSocket`]
    pub(super) struct Remote {
        pub(super) received: mpsc::UnboundedSender<Result<Datagram, Fault>>,
        pub(super) sent: mpsc::UnboundedReceiver<Datagram>,
    }

    impl UdpLayer {
        /// A layer on a scripted socket, connected to `connected` if it's set and bound otherwise
        pub(super) fn scripted(
            connected: Option<SocketAddr>,
            send_errors: &[Fault],
        ) -> (Self, Remote) {
            let (received_tx, received_rx) = mpsc::unbounded_channel();
            let (sent_tx, sent_rx) = mpsc::unbounded_channel();
            let socket = ScriptedSocket {
                received: received_rx,
                sent: sent_tx,
                send_errors: send_errors.iter().copied().collect(),
                send_attempts: 0,
                connected,
            };
            let layer = UdpLayer {
                inner: UdpType::Scripted(socket),
            };
            let remote = Remote {
                received: received_tx,
                sent: sent_rx,
            };
            (layer, remote)
        }

        /// The number of send attempts, and the number of scripted errors they didn't take
        pub(super) fn send_attempts(&self) -> (usize, usize) {
            match &self.inner {
                UdpType::Scripted(x) => (x.send_attempts, x.send_errors.len()),
                UdpType::Bound(_) | UdpType::Connected(_) => panic!("not a scripted socket"),
            }
        }
    }

    impl ScriptedSocket {
        pub(super) fn is_bound(&self) -> bool {
            self.connected.is_none()
        }

        pub(super) async fn read(
            &mut self,
            buffer: &mut [u8],
        ) -> std::io::Result<(usize, PhysAddr)> {
            let Some(received) = self.received.recv().await else {
                return std::future::pending().await;
            };
            let (data, source) = received?;
            let count = data.len().min(buffer.len());
            buffer[..count].copy_from_slice(&data[..count]);
            let source = match self.connected {
                None => PhysAddr::Udp(source),
                Some(_) => PhysAddr::None,
            };
            Ok((count, source))
        }

        pub(super) fn write(&mut self, data: &[u8], addr: PhysAddr) -> std::io::Result<usize> {
            let destination = match self.connected {
                None => destination(addr)?,
                Some(remote) => remote,
            };
            self.send_attempts = self.send_attempts.saturating_add(1);
            if let Some(fault) = self.send_errors.pop_front() {
                return Err(fault.into());
            }
            if destination.port() == 0 {
                return Err(ErrorKind::InvalidInput.into());
            }
            // the test may have dropped its receiver
            drop(self.sent.send((data.to_vec(), destination)));
            Ok(data.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scripted::{Datagram, Fault};
    use super::*;
    use crate::decode::PhysDecodeLevel;
    use crate::link::reader::LinkModes;
    use crate::link::{EndpointAddress, LinkErrorMode};
    use crate::master::task::MasterTask;
    use crate::master::MasterChannelConfig;
    use crate::session::Session;
    use crate::util::lifecycle::Enabled;
    use crate::util::test_support::{MASTER_REPLY, OUTSTATION_REQUEST};
    use std::time::Duration;

    const LOCAL: &str = "127.0.0.1:0";

    const PEER: &str = "192.0.2.1:20000";
    const PEER_PORT_0: &str = "192.0.2.1:0";

    fn peer() -> SocketAddr {
        PEER.parse().unwrap()
    }

    const LINUX: bool = cfg!(any(target_os = "linux", target_os = "android"));
    const WINDOWS: bool = cfg!(target_os = "windows");

    /// The send errors that only lose the datagram on a bound socket on this platform
    fn destination_errors() -> Vec<ErrorKind> {
        let mut kinds = vec![
            ErrorKind::AddrNotAvailable,
            ErrorKind::NetworkUnreachable,
            ErrorKind::HostUnreachable,
            ErrorKind::PermissionDenied,
        ];
        if LINUX {
            kinds.push(ErrorKind::InvalidInput);
        }
        kinds
    }

    /// Send errors that end the session on this platform
    fn socket_errors() -> Vec<ErrorKind> {
        let mut kinds = vec![
            ErrorKind::Other,
            ErrorKind::Unsupported,
            ErrorKind::OutOfMemory,
            ErrorKind::BrokenPipe,
        ];
        if !LINUX {
            kinds.push(ErrorKind::InvalidInput);
        }
        kinds
    }

    fn faults(kinds: &[ErrorKind]) -> Vec<Fault> {
        kinds.iter().copied().map(Fault::Kind).collect()
    }

    fn ttl_expired() -> std::io::Error {
        std::io::Error::from_raw_os_error(WSAENETRESET)
    }

    #[derive(Debug, PartialEq)]
    struct WriteOutcome {
        result: Result<(), ErrorKind>,
        /// send attempts, and scripted errors left over
        attempts: (usize, usize),
        sent: Vec<Datagram>,
    }

    impl WriteOutcome {
        fn new(result: Result<(), ErrorKind>, attempts: (usize, usize), sent: &[u8]) -> Self {
            Self {
                result,
                attempts,
                sent: if sent.is_empty() {
                    Vec::new()
                } else {
                    vec![(sent.to_vec(), peer())]
                },
            }
        }
    }

    /// Write to the peer from a bound socket
    async fn scripted_write(send_errors: &[ErrorKind], addr: PhysAddr) -> WriteOutcome {
        write_with(None, &faults(send_errors), addr).await
    }

    /// Write from a socket connected to the peer
    async fn connected_write(send_errors: &[ErrorKind]) -> WriteOutcome {
        write_with(Some(peer()), &faults(send_errors), PhysAddr::None).await
    }

    async fn write_with(
        connected: Option<SocketAddr>,
        send_errors: &[Fault],
        addr: PhysAddr,
    ) -> WriteOutcome {
        let (mut layer, mut remote) = UdpLayer::scripted(connected, send_errors);
        let result = layer.write_all(&[1, 2, 3], addr).await;
        let mut sent = Vec::new();
        while let Ok(x) = remote.sent.try_recv() {
            sent.push(x);
        }
        WriteOutcome {
            result: result.map_err(|err| err.kind()),
            attempts: layer.send_attempts(),
            sent,
        }
    }

    #[tokio::test]
    async fn write_loses_a_datagram_its_destination_can_not_receive() {
        for kind in destination_errors() {
            assert_eq!(
                (kind, scripted_write(&[kind], PhysAddr::Udp(peer())).await),
                (kind, WriteOutcome::new(Ok(()), (1, 0), &[]))
            );
        }
    }

    #[tokio::test]
    async fn write_fails_on_a_socket_error() {
        for kind in socket_errors() {
            assert_eq!(
                (kind, scripted_write(&[kind], PhysAddr::Udp(peer())).await),
                (kind, WriteOutcome::new(Err(kind), (1, 0), &[]))
            );
        }
    }

    #[tokio::test]
    async fn write_retries_once_after_an_icmp_error() {
        for kind in [ErrorKind::ConnectionReset, ErrorKind::ConnectionRefused] {
            assert_eq!(
                (kind, scripted_write(&[kind], PhysAddr::Udp(peer())).await),
                (kind, WriteOutcome::new(Ok(()), (2, 0), &[1, 2, 3]))
            );
        }
    }

    #[tokio::test]
    async fn write_loses_a_datagram_whose_retry_its_destination_can_not_receive() {
        for kind in destination_errors() {
            let errors = [ErrorKind::ConnectionReset, kind];
            assert_eq!(
                (kind, scripted_write(&errors, PhysAddr::Udp(peer())).await),
                (kind, WriteOutcome::new(Ok(()), (2, 0), &[]))
            );
        }
    }

    #[tokio::test]
    async fn write_fails_on_a_socket_error_in_the_retry() {
        for kind in socket_errors() {
            let errors = [ErrorKind::ConnectionReset, kind];
            assert_eq!(
                (kind, scripted_write(&errors, PhysAddr::Udp(peer())).await),
                (kind, WriteOutcome::new(Err(kind), (2, 0), &[]))
            );
        }
    }

    #[tokio::test]
    async fn write_loses_a_datagram_after_a_second_icmp_error() {
        let errors = [ErrorKind::ConnectionReset; 3];
        assert_eq!(
            scripted_write(&errors, PhysAddr::Udp(peer())).await,
            WriteOutcome::new(Ok(()), (2, 1), &[])
        );
    }

    #[tokio::test]
    async fn write_without_a_destination_fails_on_a_bound_socket() {
        assert_eq!(
            scripted_write(&[], PhysAddr::None).await,
            WriteOutcome::new(Err(ErrorKind::Other), (0, 0), &[])
        );
    }

    /// A connected socket only sends to its remote, so these errors apply to every datagram
    #[tokio::test]
    async fn write_fails_on_a_destination_error_on_a_connected_socket() {
        for kind in destination_errors() {
            assert_eq!(
                (kind, connected_write(&[kind]).await),
                (kind, WriteOutcome::new(Err(kind), (1, 0), &[]))
            );
        }
    }

    #[tokio::test]
    async fn write_retries_once_after_an_icmp_error_on_a_connected_socket() {
        assert_eq!(
            connected_write(&[ErrorKind::ConnectionRefused]).await,
            WriteOutcome::new(Ok(()), (2, 0), &[1, 2, 3])
        );
    }

    #[tokio::test]
    async fn write_loses_a_datagram_after_a_second_icmp_error_on_a_connected_socket() {
        assert_eq!(
            connected_write(&[ErrorKind::ConnectionRefused; 3]).await,
            WriteOutcome::new(Ok(()), (2, 1), &[])
        );
    }

    /// Windows reports an expired time to live as WSAENETRESET, which is otherwise uncategorized
    #[tokio::test]
    async fn write_treats_wsaenetreset_as_an_icmp_error_only_on_windows() {
        for connected in [None, Some(peer())] {
            let addr = PhysAddr::Udp(peer());
            let once = write_with(connected, &[Fault::Os(WSAENETRESET)], addr).await;
            let thrice = write_with(connected, &[Fault::Os(WSAENETRESET); 3], addr).await;
            let expected = if WINDOWS {
                (
                    WriteOutcome::new(Ok(()), (2, 0), &[1, 2, 3]),
                    WriteOutcome::new(Ok(()), (2, 1), &[]),
                )
            } else {
                let kind = ttl_expired().kind();
                (
                    WriteOutcome::new(Err(kind), (1, 0), &[]),
                    WriteOutcome::new(Err(kind), (1, 2), &[]),
                )
            };
            assert_eq!(
                (connected, once, thrice),
                (connected, expected.0, expected.1)
            );
        }
    }

    type ReadOutcome = Result<(Option<Vec<u8>>, PhysAddr), Option<i32>>;

    /// Read from a bound socket whose first read fails with the platform error `code`, and whose
    /// next read receives a datagram from the peer
    async fn read_after_os_error(code: i32) -> ReadOutcome {
        let (mut layer, remote) = UdpLayer::scripted(None, &[]);
        remote.received.send(Err(Fault::Os(code))).unwrap();
        remote.received.send(Ok((vec![2], peer()))).unwrap();

        let mut buffer = [0; 16];
        tokio::time::timeout(Duration::from_secs(1), layer.read(&mut buffer))
            .await
            .unwrap()
            .map(|(count, source)| (buffer.get(..count).map(<[u8]>::to_vec), source))
            .map_err(|err| err.raw_os_error())
    }

    /// The datagram after the error on Windows, and the error elsewhere
    fn skipped_only_on_windows(code: i32) -> ReadOutcome {
        if WINDOWS {
            Ok((Some(vec![2]), PhysAddr::Udp(peer())))
        } else {
            Err(Some(code))
        }
    }

    #[tokio::test]
    async fn read_skips_wsaenetreset_only_on_windows() {
        assert_eq!(
            read_after_os_error(WSAENETRESET).await,
            skipped_only_on_windows(WSAENETRESET)
        );
    }

    /// Windows fails the read of a datagram larger than the buffer, which other platforms truncate
    #[tokio::test]
    async fn read_skips_wsaemsgsize_only_on_windows() {
        assert_eq!(
            read_after_os_error(WSAEMSGSIZE).await,
            skipped_only_on_windows(WSAEMSGSIZE)
        );
    }

    #[tokio::test]
    async fn read_skips_a_datagram_from_port_0() {
        let (mut layer, remote) = UdpLayer::scripted(None, &[]);
        let port_0 = PEER_PORT_0.parse().unwrap();
        remote.received.send(Ok((vec![1], port_0))).unwrap();
        remote.received.send(Ok((vec![2], peer()))).unwrap();

        let mut buffer = [0; 16];
        let (count, source) = tokio::time::timeout(Duration::from_secs(1), layer.read(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (buffer.get(..count), source),
            (Some([2].as_slice()), PhysAddr::Udp(peer()))
        );
    }

    /// A request for link status from port 0 is ignored, so the reply to it can't end the session
    #[tokio::test]
    async fn master_session_ignores_a_request_from_port_0_and_answers_the_next() {
        let (layer, mut remote) = UdpLayer::scripted(None, &[]);
        let mut phys = PhysLayer::udp(layer);
        let (_channel, rx) = crate::util::channel::request_channel();
        let mut session = Session::master(MasterTask::new(
            Enabled::Yes,
            LinkModes::datagram(LinkErrorMode::Discard),
            MasterChannelConfig::new(EndpointAddress::raw(1)),
            rx,
        ));

        let port_0 = PEER_PORT_0.parse().unwrap();
        remote
            .received
            .send(Ok((OUTSTATION_REQUEST.to_vec(), port_0)))
            .unwrap();
        remote
            .received
            .send(Ok((OUTSTATION_REQUEST.to_vec(), peer())))
            .unwrap();

        tokio::select! {
            err = session.run(&mut phys) => panic!("the session ended: {err:?}"),
            sent = tokio::time::timeout(Duration::from_secs(1), remote.sent.recv()) => {
                assert_eq!(sent, Ok(Some((MASTER_REPLY.to_vec(), peer()))));
            }
        }
    }

    /// Linux reports a send to port 0 as an invalid argument
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn write_to_port_0_loses_the_datagram_and_the_socket_still_reads() {
        let socket = tokio::net::UdpSocket::bind(LOCAL).await.unwrap();
        let address = socket.local_addr().unwrap();
        let mut layer = UdpLayer::bound(socket);
        layer
            .write_all(&[1], PhysAddr::Udp("127.0.0.1:0".parse().unwrap()))
            .await
            .unwrap();

        let remote = tokio::net::UdpSocket::bind(LOCAL).await.unwrap();
        remote.send_to(&[2], address).await.unwrap();
        let mut buffer = [0; 16];
        let (count, source) = tokio::time::timeout(Duration::from_secs(1), layer.read(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (buffer.get(..count), source),
            (
                Some([2].as_slice()),
                PhysAddr::Udp(remote.local_addr().unwrap())
            )
        );
    }

    /// Connect a layer to a port that is closed once the layer has its own port, so they can't
    /// be the same, and check with a plain socket that sending to it raises an ICMP error
    async fn connected_to_closed_port() -> (UdpLayer, SocketAddr) {
        let remote = tokio::net::UdpSocket::bind(LOCAL).await.unwrap();
        let address = remote.local_addr().unwrap();
        let layer = UdpLayer::connect(LOCAL.parse().unwrap(), address)
            .await
            .unwrap();
        drop(remote);

        let probe = tokio::net::UdpSocket::bind(LOCAL).await.unwrap();
        probe.connect(address).await.unwrap();
        probe.send(&[0]).await.unwrap();
        let mut buffer = [0; 16];
        let err = tokio::time::timeout(Duration::from_secs(1), probe.recv(&mut buffer))
            .await
            .unwrap()
            .unwrap_err();
        assert!(is_icmp_error(&err), "unexpected error: {err}");

        (layer, address)
    }

    #[tokio::test]
    async fn read_ignores_a_refused_datagram() {
        let (mut layer, _) = connected_to_closed_port().await;
        layer.write_all(&[1], PhysAddr::None).await.unwrap();

        let mut buffer = [0; 16];
        let result =
            tokio::time::timeout(Duration::from_millis(200), layer.read(&mut buffer)).await;
        assert!(result.is_err(), "read should wait for data, got {result:?}");
    }

    #[tokio::test]
    async fn write_after_a_refused_datagram_is_sent() {
        let (mut layer, address) = connected_to_closed_port().await;
        layer.write_all(&[1], PhysAddr::None).await.unwrap();
        // the probe above saw its error by now, so this one has arrived too
        let remote = tokio::net::UdpSocket::bind(address).await.unwrap();
        layer.write_all(&[2], PhysAddr::None).await.unwrap();

        let mut buffer = [0; 16];
        let count = tokio::time::timeout(Duration::from_secs(1), remote.recv(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(buffer.get(..count), Some([2].as_slice()));
    }

    #[tokio::test]
    async fn read_skips_an_empty_datagram() {
        let socket = tokio::net::UdpSocket::bind(LOCAL).await.unwrap();
        let address = socket.local_addr().unwrap();
        let mut phys = PhysLayer::udp(UdpLayer::bound(socket));

        let remote = tokio::net::UdpSocket::bind(LOCAL).await.unwrap();
        remote.send_to(&[], address).await.unwrap();
        remote.send_to(&[1], address).await.unwrap();

        let mut buffer = [0; 16];
        let (count, source) = tokio::time::timeout(
            Duration::from_secs(1),
            phys.read(&mut buffer, PhysDecodeLevel::Nothing),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(buffer.get(..count), Some([1].as_slice()));
        assert_eq!(source, PhysAddr::Udp(remote.local_addr().unwrap()));
    }
}
