// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! The bind, retry, disable and shutdown sequence of a UDP master and outstation
//!
//! UDP channels have no listener, so the tests observe whether the channel holds its local port
//! by trying to bind it, and whether its task is still running. The master tests pause the clock;
//! the master has no associations, so its own retry wait is the only timer. With the clock paused,
//! the outstation's session fails and starts its retry wait before the clock moves.
//!
//! The tests of where datagrams are accepted from and sent to use the real clock, because the
//! socket modes are kernel behavior that only real sockets show.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::UdpSocket;

use crate::app::Timeout;
use crate::link::{EndpointAddress, LinkReadMode};
use crate::master::{
    AssociationConfig, AssociationHandle, AssociationHandler, AssociationInformation, Classes,
    MasterChannel, MasterChannelConfig, ReadHandler, ReadRequest,
};
use crate::outstation::{DefaultControlHandler, Feature, OutstationConfig, OutstationHandle};
use crate::udp::{OutstationUdpConfig, UdpSocketMode};
use crate::util::test_support::{
    outstation_config, watchdog, NullApplication, NullInformation, MASTER_REPLY, MASTER_REQUEST,
    NULL_UNSOLICITED_HEADER, OUTSTATION_REPLY, OUTSTATION_REQUEST,
};

const fn secs(x: u64) -> Duration {
    Duration::from_secs(x)
}

const fn millis(x: u64) -> Duration {
    Duration::from_millis(x)
}

/// A free local address, and the socket that holds it until the test drops it
///
/// A test drops the socket just before the code under test binds the address, which leaves
/// another socket only a short window to take the port.
fn reserve() -> (SocketAddr, std::net::UdpSocket) {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    (socket.local_addr().unwrap(), socket)
}

/// Whether something holds the address, found by trying to bind it
fn is_bound(address: SocketAddr) -> bool {
    std::net::UdpSocket::bind(address).is_err()
}

fn alive_tasks() -> usize {
    tokio::runtime::Handle::current()
        .metrics()
        .num_alive_tasks()
}

/// Let the spawned task run whatever it can without waiting for the clock
async fn settle() {
    tokio::time::sleep(millis(1)).await;
}

fn spawn_master(address: SocketAddr) -> MasterChannel {
    crate::udp::spawn_master_udp(
        address,
        LinkReadMode::Datagram,
        Timeout::from_secs(2).unwrap(),
        MasterChannelConfig::new(EndpointAddress::try_new(1).unwrap()),
    )
}

/// Request link status from the master as outstation 1024 and check the reply
async fn exchange_with_master(address: SocketAddr) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.send_to(&OUTSTATION_REQUEST, address).await.unwrap();
    let mut buffer = [0; 64];
    let (count, source) = socket.recv_from(&mut buffer).await.unwrap();
    assert_eq!(
        (&buffer[..count], source),
        (MASTER_REPLY.as_slice(), address)
    );
}

#[tokio::test(start_paused = true)]
async fn master_binds_when_enabled_and_retries_a_failed_bind_after_the_delay() {
    watchdog(async {
        let (address, blocker) = reserve();
        let mut channel = spawn_master(address);
        // a disabled master doesn't bind: one that tried would fail against the blocker, and bind
        // when its retry delay ends, before the delay that the enable starts
        tokio::time::sleep(secs(1)).await;

        channel.enable().await.unwrap();
        let enabled = tokio::time::Instant::now();
        settle().await;
        drop(blocker);

        tokio::time::sleep_until(enabled.checked_add(millis(1999)).unwrap()).await;
        assert!(!is_bound(address));
        tokio::time::sleep_until(enabled.checked_add(millis(2001)).unwrap()).await;
        assert!(is_bound(address));
        exchange_with_master(address).await;

        drop(channel);
        settle().await;
        assert!(!is_bound(address));
        assert_eq!(alive_tasks(), 0);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn master_disable_during_a_session_closes_the_socket_until_enabled() {
    watchdog(async {
        let (address, reservation) = reserve();
        let mut channel = spawn_master(address);
        drop(reservation);
        channel.enable().await.unwrap();
        settle().await;
        assert!(is_bound(address));

        channel.disable().await.unwrap();
        settle().await;
        assert!(!is_bound(address));
        tokio::time::sleep(secs(10)).await;
        assert!(!is_bound(address));

        channel.enable().await.unwrap();
        settle().await;
        assert!(is_bound(address));
        exchange_with_master(address).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn master_disable_during_a_retry_wait_skips_the_rest_of_the_delay() {
    watchdog(async {
        let (address, blocker) = reserve();
        let mut channel = spawn_master(address);
        channel.enable().await.unwrap();
        settle().await;
        channel.disable().await.unwrap();
        settle().await;
        drop(blocker);

        tokio::time::sleep(secs(10)).await;
        assert!(!is_bound(address));
        channel.enable().await.unwrap();
        settle().await;
        assert!(is_bound(address));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn master_shutdown_during_a_retry_wait_ends_the_task_at_once() {
    watchdog(async {
        let (address, _blocker) = reserve();
        let mut channel = spawn_master(address);
        channel.enable().await.unwrap();
        settle().await;
        assert_eq!(alive_tasks(), 1);
        drop(channel);
        settle().await;
        assert_eq!(alive_tasks(), 0);
    })
    .await;
}

fn spawn_outstation(local: SocketAddr, remote: SocketAddr) -> OutstationHandle {
    spawn_outstation_with(
        UdpSocketMode::OneToMany,
        outstation_config(1024, 1),
        local,
        remote,
    )
}

fn spawn_outstation_with(
    socket_mode: UdpSocketMode,
    config: OutstationConfig,
    local: SocketAddr,
    remote: SocketAddr,
) -> OutstationHandle {
    let udp_config = OutstationUdpConfig {
        local_endpoint: local,
        remote_endpoint: remote,
        socket_mode,
        link_read_mode: LinkReadMode::Datagram,
        retry_delay: Timeout::from_secs(2).unwrap(),
    };
    crate::udp::spawn_outstation_udp(
        udp_config,
        config,
        Box::new(NullApplication),
        Box::new(NullInformation),
        DefaultControlHandler::create(),
    )
}

#[tokio::test]
async fn outstation_binds_at_once_and_sends_to_the_remote_endpoint() {
    let (local, reservation) = reserve();
    let remote = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    drop(reservation);
    let handle = spawn_outstation(local, remote.local_addr().unwrap());

    let mut buffer = [0; 64];
    let (count, source) = tokio::time::timeout(secs(5), remote.recv_from(&mut buffer))
        .await
        .unwrap()
        .unwrap();
    assert_eq!((count, source), (17, local));
    assert_eq!(buffer[..10], NULL_UNSOLICITED_HEADER);
    assert!(is_bound(local));

    drop(handle);
    tokio::time::timeout(secs(5), async {
        while alive_tasks() > 0 {
            tokio::time::sleep(millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(!is_bound(local));
}

/// An IPv4 socket can't send to the IPv6 remote endpoint, so the session fails at its first write
#[tokio::test(start_paused = true)]
async fn outstation_closes_the_socket_before_waiting_to_retry_a_failed_session() {
    watchdog(async {
        let (local, reservation) = reserve();
        drop(reservation);
        let handle = spawn_outstation(local, "[::1]:20000".parse().unwrap());
        tokio::time::sleep(secs(1)).await;
        assert!(!is_bound(local));
        assert_eq!(alive_tasks(), 1);
        drop(handle);
    })
    .await;
}

/// A datagram larger than the read buffer doesn't end the session. Most platforms truncate it,
/// so the outstation answers the request at its start; Windows fails the read, so the outstation
/// discards all of it.
#[tokio::test]
async fn outstation_keeps_answering_after_an_oversized_datagram() {
    watchdog(async {
        let (local, reservation) = reserve();
        let master = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        drop(reservation);
        let _handle = spawn_outstation(local, master.local_addr().unwrap());
        let mut buffer = [0; 64];
        let (count, _) = master.recv_from(&mut buffer).await.unwrap();
        assert_eq!(
            (count, buffer.get(..10)),
            (17, Some(NULL_UNSOLICITED_HEADER.as_slice()))
        );

        // larger than the read buffer for the default fragment size
        let mut oversized = MASTER_REQUEST.to_vec();
        oversized.resize(4096, 0);
        master.send_to(&oversized, local).await.unwrap();
        master.send_to(&MASTER_REQUEST, local).await.unwrap();

        let replies = if cfg!(windows) { 1 } else { 2 };
        for _ in 0..replies {
            let (count, source) = master.recv_from(&mut buffer).await.unwrap();
            assert_eq!(
                (buffer.get(..count), source),
                (Some(OUTSTATION_REPLY.as_slice()), local)
            );
        }
    })
    .await;
}

/// Receive the null unsolicited response that the outstation at `outstation` sends when it starts
async fn expect_null_unsolicited(socket: &UdpSocket, outstation: SocketAddr) {
    let mut buffer = [0; 64];
    let (count, source) = socket.recv_from(&mut buffer).await.unwrap();
    assert_eq!(
        (count, buffer.get(..10), source),
        (17, Some(NULL_UNSOLICITED_HEADER.as_slice()), outstation)
    );
}

/// Request link status from the outstation at `outstation` as master 1 and check the reply
async fn exchange_with_outstation(socket: &UdpSocket, outstation: SocketAddr) {
    socket.send_to(&MASTER_REQUEST, outstation).await.unwrap();
    let mut buffer = [0; 64];
    let (count, source) = socket.recv_from(&mut buffer).await.unwrap();
    assert_eq!(
        (buffer.get(..count), source),
        (Some(OUTSTATION_REPLY.as_slice()), outstation)
    );
}

/// The socket has no datagram waiting
///
/// The outstation answers requests in order, and loopback delivers a datagram before its send
/// returns, so after the answer to a later request this shows that an earlier one wasn't answered.
fn assert_nothing_received(socket: UdpSocket) {
    let socket = socket.into_std().unwrap();
    let mut buffer = [0; 64];
    assert_eq!(
        socket.recv_from(&mut buffer).map_err(|err| err.kind()),
        Err(std::io::ErrorKind::WouldBlock)
    );
}

#[tokio::test]
async fn one_to_one_outstation_ignores_datagrams_from_other_addresses() {
    watchdog(async {
        let (local, reservation) = reserve();
        let master = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let other = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        drop(reservation);
        let _handle = spawn_outstation_with(
            UdpSocketMode::OneToOne,
            outstation_config(1024, 1),
            local,
            master.local_addr().unwrap(),
        );
        expect_null_unsolicited(&master, local).await;

        other.send_to(&MASTER_REQUEST, local).await.unwrap();
        exchange_with_outstation(&master, local).await;
        assert_nothing_received(other);
    })
    .await;
}

#[tokio::test]
async fn one_to_many_outstation_answers_each_requester_at_its_address() {
    watchdog(async {
        let (local, reservation) = reserve();
        let remote = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let first = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let second = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        drop(reservation);
        let _handle = spawn_outstation(local, remote.local_addr().unwrap());
        expect_null_unsolicited(&remote, local).await;

        exchange_with_outstation(&first, local).await;
        exchange_with_outstation(&second, local).await;
        // the remote endpoint is only for unsolicited responses
        assert_nothing_received(remote);
    })
    .await;
}

struct NullReadHandler;
impl ReadHandler for NullReadHandler {}

struct NullAssociationHandler;
impl AssociationHandler for NullAssociationHandler {}

struct NullAssociationInformation;
impl AssociationInformation for NullAssociationInformation {}

/// Outstation 1024 at `local`, answering master 1 and sending no unsolicited responses
fn spawn_quiet_outstation(local: SocketAddr, master: SocketAddr) -> OutstationHandle {
    let mut config = outstation_config(1024, 1);
    config.features.unsolicited = Feature::Disabled;
    spawn_outstation_with(UdpSocketMode::OneToMany, config, local, master)
}

/// An enabled master at `local` with a quiet association to outstation 1024 at `destination`
async fn spawn_associated_master(
    local: SocketAddr,
    destination: SocketAddr,
) -> (MasterChannel, AssociationHandle) {
    let mut channel = spawn_master(local);
    channel.enable().await.unwrap();
    let association = channel
        .add_udp_association(
            EndpointAddress::try_new(1024).unwrap(),
            destination,
            AssociationConfig::quiet(),
            Box::new(NullReadHandler),
            Box::new(NullAssociationHandler),
            Box::new(NullAssociationInformation),
        )
        .await
        .unwrap();
    (channel, association)
}

fn class0_read() -> ReadRequest {
    ReadRequest::class_scan(Classes::class0())
}

#[tokio::test]
async fn master_udp_association_reads_from_its_destination() {
    watchdog(async {
        let (master, master_reservation) = reserve();
        let (outstation, outstation_reservation) = reserve();
        drop((master_reservation, outstation_reservation));
        let _outstation = spawn_quiet_outstation(outstation, master);
        let (_channel, mut association) = spawn_associated_master(master, outstation).await;
        assert_eq!(association.read(class0_read()).await, Ok(()));
    })
    .await;
}

/// The master matches a solicited response by its link address alone, not by the address it came
/// from, so a response relayed from another port completes the request
#[tokio::test]
async fn master_udp_association_accepts_a_response_from_another_port() {
    watchdog(async {
        let (master, master_reservation) = reserve();
        let (outstation, outstation_reservation) = reserve();
        let destination = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let relay = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        drop((master_reservation, outstation_reservation));
        let _outstation = spawn_quiet_outstation(outstation, master);
        let (_channel, mut association) =
            spawn_associated_master(master, destination.local_addr().unwrap()).await;

        let forward = async {
            let mut buffer = [0; 2048];
            let (count, source) = destination.recv_from(&mut buffer).await.unwrap();
            assert_eq!(source, master);
            relay.send_to(&buffer[..count], outstation).await.unwrap();
            let (count, source) = relay.recv_from(&mut buffer).await.unwrap();
            assert_eq!(source, outstation);
            relay.send_to(&buffer[..count], master).await.unwrap();
        };
        let ((), result) = tokio::join!(forward, association.read(class0_read()));
        assert_eq!(result, Ok(()));
    })
    .await;
}
