// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::decode::PhysDecodeLevel;
use crate::link::constant::LINK_HEADER_LENGTH;
use crate::link::error::LogicError;
use std::io::ErrorKind;
use std::net::SocketAddr;

use crate::udp::layer::UdpLayer;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// A connected stream that a TCP or TLS server session runs over
///
/// Production uses only `TcpStream`. Tests also use in-memory pipes, which wake the peer directly,
/// so that a paused clock doesn't advance while data is in flight.
pub(crate) trait Socket: AsyncRead + AsyncWrite + Unpin + Send + 'static {
    /// Enable or disable `TCP_NODELAY`
    fn set_nodelay(&self, value: bool) -> std::io::Result<()>;

    /// A physical layer that runs a session over this stream
    fn into_phys(self) -> PhysLayer;

    /// A physical layer that runs a session over TLS on this stream
    #[cfg(feature = "enable-tls")]
    fn into_tls_phys(stream: tokio_rustls::TlsStream<Self>) -> PhysLayer
    where
        Self: Sized;
}

impl Socket for tokio::net::TcpStream {
    fn set_nodelay(&self, value: bool) -> std::io::Result<()> {
        tokio::net::TcpStream::set_nodelay(self, value)
    }

    fn into_phys(self) -> PhysLayer {
        PhysLayer::tcp(self)
    }

    #[cfg(feature = "enable-tls")]
    fn into_tls_phys(stream: tokio_rustls::TlsStream<Self>) -> PhysLayer {
        PhysLayer::tls(stream)
    }
}

#[cfg(test)]
impl Socket for tokio::io::DuplexStream {
    fn set_nodelay(&self, _: bool) -> std::io::Result<()> {
        Ok(())
    }

    fn into_phys(self) -> PhysLayer {
        PhysLayer::duplex(self)
    }

    #[cfg(feature = "enable-tls")]
    fn into_tls_phys(stream: tokio_rustls::TlsStream<Self>) -> PhysLayer {
        PhysLayer::new(PhysIo::TlsDuplex(Box::new(stream)))
    }
}

/// Source or destination at the physical layer from which a link frame was read/written
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum PhysAddr {
    /// The default type used for everything other than UDP
    None,
    /// Frames carried over UDP come to/from a SocketAddr
    Udp(SocketAddr),
}

/// The physical layers a session can run over
pub(crate) enum PhysIo {
    Tcp(tokio::net::TcpStream),
    Udp(UdpLayer),
    /// TLS type is boxed because its size is huge
    #[cfg(feature = "enable-tls")]
    Tls(Box<tokio_rustls::TlsStream<tokio::net::TcpStream>>),
    #[cfg(feature = "serial")]
    Serial(tokio_serial::SerialStream),
    #[cfg(test)]
    Mock(sfio_tokio_mock_io::Mock),
    /// a stream with a bounded buffer, so writes can wait on the peer
    #[cfg(test)]
    Duplex(tokio::io::DuplexStream),
    /// TLS over a [`Self::Duplex`] stream, flushed like [`Self::Tls`]
    #[cfg(all(test, feature = "enable-tls"))]
    TlsDuplex(Box<tokio_rustls::TlsStream<tokio::io::DuplexStream>>),
}

/// A physical layer, and the link-layer reply it is writing
///
/// The link layer replies to a frame inside a read that callers race against timers and drop.
/// The reply is queued here and written by [`PhysLayer::finish_reply`], which every later read of
/// a frame and every write calls first, so dropping the read neither loses the reply nor leaves
/// part of it ahead of other bytes.
pub(crate) struct PhysLayer {
    io: PhysIo,
    reply: PendingReply,
}

/// The progress of writing a link-layer reply
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum PendingReply {
    None,
    /// `frame[written..]` has yet to be written to `addr`
    Write {
        frame: [u8; LINK_HEADER_LENGTH],
        addr: PhysAddr,
        written: usize,
    },
    /// the frame was written to a stream that must still be flushed
    Flush,
}

impl PendingReply {
    /// Record that `count` more bytes were written, moving on to a flush if the stream needs one
    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    fn wrote(&mut self, count: usize, flush: bool) {
        if let Self::Write { frame, written, .. } = self {
            *written = written.saturating_add(count);
            if *written < frame.len() {
                return;
            }
        }
        *self = if flush { Self::Flush } else { Self::None };
    }

    /// Write the rest of the reply to a stream, flushing it afterwards if `flush` is set
    ///
    /// Each step is a single write or flush, which a dropped future either made or didn't, so
    /// calling this again resumes where the last call stopped.
    async fn write_to<S>(&mut self, stream: &mut S, flush: bool) -> Result<(), std::io::Error>
    where
        S: AsyncWrite + Unpin,
    {
        loop {
            match self {
                Self::None => return Ok(()),
                Self::Write { frame, written, .. } => {
                    let Some(rest) = frame.get(*written..).filter(|rest| !rest.is_empty()) else {
                        self.wrote(0, flush);
                        continue;
                    };
                    let count = stream.write(rest).await?;
                    if count == 0 {
                        return Err(ErrorKind::WriteZero.into());
                    }
                    self.wrote(count, flush);
                }
                Self::Flush => {
                    stream.flush().await?;
                    *self = Self::None;
                }
            }
        }
    }

    /// Send the reply as one datagram, which goes out whole or not at all
    async fn send_to(&mut self, socket: &mut UdpLayer) -> Result<(), std::io::Error> {
        if let Self::Write { frame, addr, .. } = self {
            socket.write_all(frame, *addr).await?;
        }
        *self = Self::None;
        Ok(())
    }
}

impl std::fmt::Debug for PhysLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.io {
            PhysIo::Tcp(_) => f.write_str("Tcp"),
            PhysIo::Udp(_) => f.write_str("Udp"),
            #[cfg(feature = "enable-tls")]
            PhysIo::Tls(_) => f.write_str("Tls"),
            #[cfg(feature = "serial")]
            PhysIo::Serial(_) => f.write_str("Serial"),
            #[cfg(test)]
            PhysIo::Mock(_) => f.write_str("Mock"),
            #[cfg(test)]
            PhysIo::Duplex(_) => f.write_str("Duplex"),
            #[cfg(all(test, feature = "enable-tls"))]
            PhysIo::TlsDuplex(_) => f.write_str("TlsDuplex"),
        }
    }
}

impl PhysLayer {
    fn new(io: PhysIo) -> Self {
        Self {
            io,
            reply: PendingReply::None,
        }
    }

    pub(crate) fn tcp(stream: tokio::net::TcpStream) -> Self {
        Self::new(PhysIo::Tcp(stream))
    }

    pub(crate) fn udp(layer: UdpLayer) -> Self {
        Self::new(PhysIo::Udp(layer))
    }

    #[cfg(feature = "enable-tls")]
    pub(crate) fn tls(stream: tokio_rustls::TlsStream<tokio::net::TcpStream>) -> Self {
        Self::new(PhysIo::Tls(Box::new(stream)))
    }

    #[cfg(feature = "serial")]
    pub(crate) fn serial(stream: tokio_serial::SerialStream) -> Self {
        Self::new(PhysIo::Serial(stream))
    }

    #[cfg(test)]
    pub(crate) fn mock(mock: sfio_tokio_mock_io::Mock) -> Self {
        Self::new(PhysIo::Mock(mock))
    }

    #[cfg(test)]
    pub(crate) fn duplex(stream: tokio::io::DuplexStream) -> Self {
        Self::new(PhysIo::Duplex(stream))
    }

    #[cfg(test)]
    pub(crate) fn io(&self) -> &PhysIo {
        &self.io
    }

    /// Whether reads return whole datagrams, truncating any that don't fit
    pub(crate) fn is_datagram_socket(&self) -> bool {
        matches!(self.io, PhysIo::Udp(_))
    }

    /// Queue a link-layer reply, which [`Self::finish_reply`] writes
    ///
    /// Fails if the last reply hasn't been finished.
    pub(crate) fn queue_reply(
        &mut self,
        frame: [u8; LINK_HEADER_LENGTH],
        addr: PhysAddr,
        level: PhysDecodeLevel,
    ) -> Result<(), LogicError> {
        if self.reply != PendingReply::None {
            return Err(LogicError::BadWrite);
        }
        if level.enabled() {
            tracing::info!("PHYS TX - {}", PhysDisplay::new(level, &frame));
        }
        self.reply = PendingReply::Write {
            frame,
            addr,
            written: 0,
        };
        Ok(())
    }

    /// Write the rest of a queued reply, if there is one
    ///
    /// The future can be dropped and called again without losing or repeating any bytes. A reply
    /// that fails to write is abandoned, since the error ends the session.
    pub(crate) async fn finish_reply(&mut self) -> Result<(), std::io::Error> {
        let reply = &mut self.reply;
        let result = match &mut self.io {
            PhysIo::Tcp(x) => reply.write_to(x, false).await,
            PhysIo::Udp(x) => reply.send_to(x).await,
            #[cfg(feature = "enable-tls")]
            PhysIo::Tls(x) => reply.write_to(x.as_mut(), true).await,
            #[cfg(feature = "serial")]
            PhysIo::Serial(x) => reply.write_to(x, false).await,
            #[cfg(test)]
            PhysIo::Mock(x) => reply.write_to(x, false).await,
            #[cfg(test)]
            PhysIo::Duplex(x) => reply.write_to(x, false).await,
            #[cfg(all(test, feature = "enable-tls"))]
            PhysIo::TlsDuplex(x) => reply.write_to(x.as_mut(), true).await,
        };
        if result.is_err() {
            *reply = PendingReply::None;
        }
        result
    }

    pub(crate) async fn read(
        &mut self,
        buffer: &mut [u8],
        level: PhysDecodeLevel,
    ) -> Result<(usize, PhysAddr), std::io::Error> {
        let (length, addr) = match &mut self.io {
            PhysIo::Tcp(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
            PhysIo::Udp(x) => x.read(buffer).await?,
            #[cfg(feature = "enable-tls")]
            PhysIo::Tls(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
            #[cfg(feature = "serial")]
            PhysIo::Serial(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
            #[cfg(test)]
            PhysIo::Mock(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
            #[cfg(test)]
            PhysIo::Duplex(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
            #[cfg(all(test, feature = "enable-tls"))]
            PhysIo::TlsDuplex(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
        };

        if length == 0 {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "read return 0",
            ));
        }

        if level.enabled() {
            if let Some(x) = buffer.get(0..length) {
                tracing::info!("PHYS RX - {}", PhysDisplay::new(level, x))
            }
        }

        Ok((length, addr))
    }

    pub(crate) async fn write(
        &mut self,
        data: &[u8],
        addr: PhysAddr,
        level: PhysDecodeLevel,
    ) -> Result<(), std::io::Error> {
        self.finish_reply().await?;

        if level.enabled() {
            tracing::info!("PHYS TX - {}", PhysDisplay::new(level, data));
        }

        match &mut self.io {
            PhysIo::Tcp(x) => x.write_all(data).await,
            PhysIo::Udp(x) => x.write_all(data, addr).await,
            #[cfg(feature = "enable-tls")]
            PhysIo::Tls(x) => write_all_and_flush(x, data).await,
            #[cfg(feature = "serial")]
            PhysIo::Serial(x) => x.write_all(data).await,
            #[cfg(test)]
            PhysIo::Mock(x) => x.write_all(data).await,
            #[cfg(test)]
            PhysIo::Duplex(x) => x.write_all(data).await,
            #[cfg(all(test, feature = "enable-tls"))]
            PhysIo::TlsDuplex(x) => write_all_and_flush(x, data).await,
        }
    }
}

/// Write all of the data and then flush the stream
///
/// tokio-rustls returns from a write once rustls has accepted the plaintext, even if the
/// resulting ciphertext could not yet be written to the socket. Nothing sends the buffered
/// ciphertext until the next write or flush, so TLS writes must be flushed.
#[cfg(feature = "enable-tls")]
async fn write_all_and_flush<S>(stream: &mut S, data: &[u8]) -> Result<(), std::io::Error>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    stream.write_all(data).await?;
    stream.flush().await
}

pub(crate) struct PhysDisplay<'a> {
    level: PhysDecodeLevel,
    data: &'a [u8],
}

impl<'a> PhysDisplay<'a> {
    pub(crate) fn new(level: PhysDecodeLevel, data: &'a [u8]) -> Self {
        PhysDisplay { level, data }
    }
}

impl std::fmt::Display for PhysDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} bytes", self.data.len())?;
        if self.level.data_enabled() {
            crate::util::decode::format_bytes(f, self.data)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod reply_tests {
    use super::{PendingReply, PhysAddr, PhysDecodeLevel, PhysLayer};

    use std::future::Future;
    use std::pin::{pin, Pin};
    use std::task::{Context, Poll, Waker};

    use crate::util::test_support::{watchdog, OUTSTATION_REPLY};

    fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }

    mod properties {
        use super::*;
        use crate::link::constant::LINK_HEADER_LENGTH;

        use std::collections::VecDeque;

        use proptest::prelude::*;
        use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

        type Frame = [u8; LINK_HEADER_LENGTH];

        /// A stream that accepts as many bytes per write as its script says and completes or
        /// stalls each flush as scripted, then accepts everything once the script runs out
        #[derive(Default)]
        struct ScriptedStream {
            /// how many bytes each write accepts, where 0 stalls the write
            writes: VecDeque<usize>,
            /// whether each flush completes
            flushes: VecDeque<bool>,
            received: Vec<u8>,
            /// how many bytes had been received when a flush last completed
            flushed: Option<usize>,
        }

        impl AsyncWrite for ScriptedStream {
            fn poll_write(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<std::io::Result<usize>> {
                let this = self.get_mut();
                match this.writes.pop_front() {
                    Some(0) => Poll::Pending,
                    capacity => {
                        let count = capacity.map_or(buf.len(), |x| x.min(buf.len()));
                        this.received.extend(buf.iter().take(count));
                        Poll::Ready(Ok(count))
                    }
                }
            }

            fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
                let this = self.get_mut();
                if this.flushes.pop_front() == Some(false) {
                    return Poll::Pending;
                }
                this.flushed = Some(this.received.len());
                Poll::Ready(Ok(()))
            }

            fn poll_shutdown(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
            ) -> Poll<std::io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        /// Whether the reply's progress agrees with what the stream received: the stream holds
        /// exactly the bytes the reply counts as written, in order and once each, and a reply
        /// to a stream that must be flushed isn't finished until a flush follows its last byte
        fn consistent(
            reply: PendingReply,
            frame: Frame,
            stream: &ScriptedStream,
            flush: bool,
        ) -> bool {
            let complete = stream.received == frame;
            match reply {
                PendingReply::Write {
                    frame: queued,
                    written,
                    ..
                } => queued == frame && frame.get(..written) == Some(stream.received.as_slice()),
                PendingReply::Flush => flush && complete,
                PendingReply::None => {
                    complete && (!flush || stream.flushed == Some(LINK_HEADER_LENGTH))
                }
            }
        }

        /// Run a case on its own runtime under the [`watchdog`], returning its result to proptest
        ///
        /// The cases poll with a no-op waker, so a call that loops forever inside one poll never
        /// yields; the watchdog then aborts the process rather than letting the test hang. The
        /// case runs unconstrained, so tokio's cooperative budget can't stall the duplex, which
        /// the case polls many times without yielding.
        fn guarded(case: impl FnOnce() -> Result<(), TestCaseError>) -> Result<(), TestCaseError> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .map_err(|err| TestCaseError::fail(err.to_string()))?;
            runtime.block_on(async {
                let mut result = Ok(());
                watchdog(tokio::task::unconstrained(async { result = case() })).await;
                result
            })
        }

        /// bytes a write accepts, weighted toward stalls, single bytes and the frame's length
        fn capacity() -> impl Strategy<Value = usize> {
            prop_oneof![
                2 => Just(0),
                3 => Just(1),
                3 => 2usize..9,
                // around the 10-byte frame
                2 => prop::sample::select(vec![9usize, 10, 11]),
                1 => 12usize..64,
            ]
        }

        /// how many bytes each write accepts, mostly stalling after a write that made progress
        fn writes() -> impl Strategy<Value = VecDeque<usize>> {
            prop::collection::vec((capacity(), prop::bool::weighted(0.6)), 0..16).prop_map(|x| {
                x.into_iter()
                    .flat_map(|(capacity, stall)| [Some(capacity), stall.then_some(0)])
                    .flatten()
                    .collect()
            })
        }

        /// how many times a call is polled before it's dropped, mostly once
        fn polls() -> impl Strategy<Value = usize> {
            prop_oneof![
                1 => Just(0),
                4 => Just(1),
                2 => 2usize..=4,
            ]
        }

        fn resume_after_cancellations(
            frame: Frame,
            flush: bool,
            writes: VecDeque<usize>,
            flushes: Vec<bool>,
            attempts: Vec<usize>,
        ) -> Result<(), TestCaseError> {
            let mut stream = ScriptedStream {
                writes,
                flushes: flushes.into(),
                ..Default::default()
            };
            let mut reply = PendingReply::Write {
                frame,
                addr: PhysAddr::None,
                written: 0,
            };
            for polls in attempts {
                let mut finished = false;
                {
                    let mut write = pin!(reply.write_to(&mut stream, flush));
                    for _ in 0..polls {
                        if let Poll::Ready(result) = poll_once(write.as_mut()) {
                            prop_assert!(result.is_ok());
                            finished = true;
                            break;
                        }
                    }
                }
                prop_assert!(
                    consistent(reply, frame, &stream, flush),
                    "{:?} {:?} {:?}",
                    reply,
                    stream.received,
                    stream.flushed
                );
                if finished {
                    prop_assert_eq!(reply, PendingReply::None);
                }
            }

            // once the stream stops stalling, the rest of the reply goes out
            stream.writes.clear();
            stream.flushes.clear();
            let result = {
                let mut write = pin!(reply.write_to(&mut stream, flush));
                (0..3).find_map(|_| match poll_once(write.as_mut()) {
                    Poll::Ready(result) => Some(result.is_ok()),
                    Poll::Pending => None,
                })
            };
            prop_assert_eq!(result, Some(true));
            prop_assert_eq!(stream.received.as_slice(), &frame[..]);
            prop_assert_eq!(reply, PendingReply::None);
            prop_assert!(consistent(reply, frame, &stream, flush));
            Ok(())
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]

            /// Writing a reply to a stream that takes it in arbitrary parts, with each call
            /// cancelled after an arbitrary number of polls, sends every byte once and in order,
            /// and a stream that needs it is flushed after the last byte
            #[test]
            fn a_reply_resumes_where_a_cancelled_write_stopped(
                frame in any::<Frame>(),
                flush in any::<bool>(),
                writes in writes(),
                flushes in prop::collection::vec(prop::bool::weighted(0.3), 0..6),
                attempts in prop::collection::vec(polls(), 0..12),
            ) {
                guarded(|| resume_after_cancellations(frame, flush, writes, flushes, attempts))?;
            }
        }

        #[derive(Copy, Clone, Debug)]
        enum Step {
            /// poll `finish_reply` this many times, then drop it
            Attempt(usize),
            /// the peer reads up to this many bytes
            Drain(usize),
        }

        fn step() -> impl Strategy<Value = Step> {
            prop_oneof![
                (0usize..=3).prop_map(Step::Attempt),
                (1usize..=12).prop_map(Step::Drain),
            ]
        }

        /// read up to `max` bytes the peer has been sent, without waiting
        fn drain(peer: &mut DuplexStream, max: usize, received: &mut Vec<u8>) {
            let mut buffer = [0; 16];
            let mut buf = ReadBuf::new(&mut buffer[..max.min(16)]);
            let poll = Pin::new(peer).poll_read(&mut Context::from_waker(Waker::noop()), &mut buf);
            if let Poll::Ready(Ok(())) = poll {
                received.extend_from_slice(buf.filled());
            }
        }

        fn write_after_cancellations(
            capacity: usize,
            frame: Frame,
            steps: Vec<Step>,
            data: Vec<u8>,
        ) -> Result<(), TestCaseError> {
            let (local, mut peer) = tokio::io::duplex(capacity);
            let mut phys = PhysLayer::duplex(local);
            prop_assert!(phys
                .queue_reply(frame, PhysAddr::None, PhysDecodeLevel::Nothing)
                .is_ok());

            let mut received = Vec::new();
            for step in steps {
                match step {
                    Step::Attempt(polls) => {
                        let mut finish = pin!(phys.finish_reply());
                        for _ in 0..polls {
                            if let Poll::Ready(result) = poll_once(finish.as_mut()) {
                                prop_assert!(result.is_ok());
                                break;
                            }
                        }
                    }
                    Step::Drain(max) => drain(&mut peer, max, &mut received),
                }
                prop_assert!(frame.starts_with(&received), "{:?}", received);
            }

            {
                // each stalled poll is followed by a read that frees room
                let mut write = pin!(phys.write(&data, PhysAddr::None, PhysDecodeLevel::Nothing));
                let mut finished = false;
                for _ in 0..64 {
                    if let Poll::Ready(result) = poll_once(write.as_mut()) {
                        prop_assert!(result.is_ok());
                        finished = true;
                        break;
                    }
                    drain(&mut peer, 16, &mut received);
                }
                prop_assert!(finished);
            }
            for _ in 0..2 {
                drain(&mut peer, 16, &mut received);
            }

            prop_assert_eq!(received, [&frame[..], &data[..]].concat());
            prop_assert_eq!(phys.reply, PendingReply::None);
            Ok(())
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(128))]

            /// However often finishing a reply to a stream with little room is cancelled, a
            /// following write sends the rest of the reply before its own data
            #[test]
            fn a_write_follows_the_whole_reply(
                capacity in 1usize..=12,
                frame in any::<Frame>(),
                steps in prop::collection::vec(step(), 0..16),
                data in prop::collection::vec(any::<u8>(), 1..=20),
            ) {
                guarded(|| write_after_cancellations(capacity, frame, steps, data))?;
            }
        }

        /// A stream that takes 4 bytes per write gets the 10-byte reply in three parts, the
        /// first two cut off by cancellations, and then the data written after it
        #[test]
        fn reply_resumes_after_cancelled_partial_writes_to_a_stream() {
            let writes = VecDeque::from([4, 0, 4, 0]);
            guarded(|| {
                resume_after_cancellations(OUTSTATION_REPLY, false, writes, vec![], vec![1, 1])
            })
            .unwrap();
            let steps = vec![Step::Attempt(1), Step::Drain(4), Step::Attempt(1)];
            guarded(|| {
                write_after_cancellations(4, OUTSTATION_REPLY, steps, vec![0xAA, 0xBB, 0xCC])
            })
            .unwrap();
        }

        /// The consistency check rejects a reply that lost track of bytes it wrote, wrote bytes
        /// twice or skipped a flush
        #[test]
        fn consistency_rejects_lost_repeated_and_unflushed_bytes() {
            let frame = OUTSTATION_REPLY;
            let stream = |received: &[u8], flushed| ScriptedStream {
                received: received.to_vec(),
                flushed,
                ..Default::default()
            };
            let written = |written| PendingReply::Write {
                frame,
                addr: PhysAddr::None,
                written,
            };

            let repeated = [&frame[..4], &frame[..4]].concat();
            let none = PendingReply::None;
            let flush = PendingReply::Flush;
            // (reply, bytes received, received when last flushed, must flush, consistent)
            let cases = [
                (written(4), &frame[..4], None, false, true),
                // resuming would skip or repeat bytes
                (written(4), &frame[..8], None, false, false),
                (written(8), &frame[..4], None, false, false),
                (written(8), &repeated[..], None, false, false),
                // a stream that needs flushing gets a flush after the last byte
                (none, &frame[..], None, false, true),
                (none, &frame[..], None, true, false),
                (none, &frame[..], Some(8), true, false),
                (none, &frame[..], Some(10), true, true),
                (flush, &frame[..], None, false, false),
                (flush, &frame[..8], None, true, false),
            ];
            for (reply, received, flushed, must_flush, expected) in cases {
                assert_eq!(
                    consistent(reply, frame, &stream(received, flushed), must_flush),
                    expected,
                    "{reply:?} {received:?} {flushed:?} {must_flush}"
                );
            }
        }
    }
}

#[cfg(test)]
#[cfg(feature = "enable-tls")]
mod tls_tests {
    use super::{
        write_all_and_flush, PendingReply, PhysAddr, PhysDecodeLevel, PhysIo, Socket,
        LINK_HEADER_LENGTH,
    };

    use std::future::Future;
    use std::net::Ipv4Addr;
    use std::pin::{pin, Pin};
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
    use tokio_rustls::rustls::pki_types::{IpAddr, ServerName};
    use tokio_rustls::{client, server, TlsAcceptor, TlsConnector};

    use crate::tcp::tls::MinTlsVersion;
    use crate::util::test_support::tls::cert;
    use crate::util::test_support::{MASTER_REPLY, OUTSTATION_REPLY};

    // small enough that a single frame's ciphertext can't fit
    const DUPLEX_CAPACITY: usize = 64;
    const FRAME: [u8; 292] = [0xAB; 292];

    fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }

    async fn with_timeout<F: Future>(future: F) -> F::Output {
        tokio::time::timeout(Duration::from_secs(5), future)
            .await
            .expect("timed out")
    }

    async fn connect() -> (
        client::TlsStream<DuplexStream>,
        server::TlsStream<DuplexStream>,
    ) {
        let client_config = sfio_rustls_config::client::self_signed(
            MinTlsVersion::V12.into(),
            &cert("entity2_cert.pem"),
            &cert("entity1_cert.pem"),
            &cert("entity1_key.pem"),
            None,
        )
        .unwrap();

        let mut server_config = sfio_rustls_config::server::self_signed(
            MinTlsVersion::V12.into(),
            &cert("entity1_cert.pem"),
            &cert("entity2_cert.pem"),
            &cert("entity2_key.pem"),
            None,
        )
        .unwrap();
        // session tickets are written after the client handshake completes and would
        // fill the duplex, stalling the server handshake
        server_config.send_tls13_tickets = 0;

        let (client_io, server_io) = tokio::io::duplex(DUPLEX_CAPACITY);
        let connector = TlsConnector::from(Arc::new(client_config));
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let server_name = ServerName::IpAddress(IpAddr::V4(Ipv4Addr::UNSPECIFIED.into()));

        let (client, server) = with_timeout(async {
            tokio::join!(
                connector.connect(server_name, client_io),
                acceptor.accept(server_io)
            )
        })
        .await;

        (client.unwrap(), server.unwrap())
    }

    #[tokio::test]
    async fn write_all_alone_can_leave_ciphertext_buffered() {
        let (mut client, _server) = connect().await;

        // the server isn't reading, yet the write completes immediately
        assert!(matches!(
            poll_once(pin!(client.write_all(&FRAME))),
            Poll::Ready(Ok(()))
        ));
        assert!(client.get_ref().1.wants_write());
    }

    #[tokio::test]
    async fn write_all_and_flush_completes_once_ciphertext_is_written() {
        let (mut client, mut server) = connect().await;

        let mut received = [0; FRAME.len()];
        {
            let mut write = pin!(write_all_and_flush(&mut client, &FRAME));
            assert!(poll_once(write.as_mut()).is_pending());

            let (write_result, read_result) =
                with_timeout(async { tokio::join!(write, server.read_exact(&mut received)) }).await;
            write_result.unwrap();
            read_result.unwrap();
        }

        assert_eq!(received, FRAME);
        assert!(!client.get_ref().1.wants_write());
    }

    #[tokio::test]
    async fn write_all_and_flush_fails_if_peer_closes_before_flush() {
        let (mut client, server) = connect().await;

        let mut write = pin!(write_all_and_flush(&mut client, &FRAME));
        assert!(poll_once(write.as_mut()).is_pending());

        drop(server);

        assert!(with_timeout(write).await.is_err());
    }

    #[tokio::test]
    async fn reply_resumes_a_cancelled_flush() {
        let (mut client, mut server) = connect().await;
        // ciphertext the server hasn't read fills the duplex
        assert!(matches!(
            poll_once(pin!(client.write_all(&FRAME))),
            Poll::Ready(Ok(()))
        ));

        let frame = [0x05, 0x64, 0x05, 0x00, 0x01, 0x00, 0x00, 0x04, 0x27, 0x70];
        let mut reply = PendingReply::Write {
            frame,
            addr: PhysAddr::None,
            written: 0,
        };
        assert!(poll_once(pin!(reply.write_to(&mut client, true))).is_pending());
        assert_eq!(reply, PendingReply::Flush);

        let mut received = [0; FRAME.len() + LINK_HEADER_LENGTH];
        let (write_result, read_result) = with_timeout(async {
            tokio::join!(
                reply.write_to(&mut client, true),
                server.read_exact(&mut received)
            )
        })
        .await;
        write_result.unwrap();
        read_result.unwrap();

        assert_eq!(received.as_slice(), [&FRAME[..], &frame].concat());
        assert_eq!(reply, PendingReply::None);
        assert!(!client.get_ref().1.wants_write());
    }

    #[tokio::test]
    async fn tls_over_a_duplex_flushes_replies_and_writes_through_the_phys_layer() {
        let (mut client, mut server) = connect().await;
        // ciphertext the server hasn't read fills the duplex
        assert!(matches!(
            poll_once(pin!(client.write_all(&FRAME))),
            Poll::Ready(Ok(()))
        ));
        let mut phys = DuplexStream::into_tls_phys(tokio_rustls::TlsStream::from(client));
        assert!(matches!(phys.io(), PhysIo::TlsDuplex(_)));

        // the reply is accepted, but can't be flushed until the server reads
        phys.queue_reply(OUTSTATION_REPLY, PhysAddr::None, PhysDecodeLevel::Nothing)
            .unwrap();
        assert!(poll_once(pin!(phys.finish_reply())).is_pending());
        assert_eq!(phys.reply, PendingReply::Flush);

        let mut received = [0; FRAME.len() + LINK_HEADER_LENGTH];
        let (reply_result, read_result) = with_timeout(async {
            tokio::join!(phys.finish_reply(), server.read_exact(&mut received))
        })
        .await;
        reply_result.unwrap();
        read_result.unwrap();
        assert_eq!(
            received.as_slice(),
            [&FRAME[..], &OUTSTATION_REPLY].concat()
        );
        assert_eq!(phys.reply, PendingReply::None);

        // a write whose ciphertext doesn't fit completes once the server has read all of it
        let mut received = [0; FRAME.len()];
        let (write_result, read_result) = with_timeout(async {
            tokio::join!(
                phys.write(&FRAME, PhysAddr::None, PhysDecodeLevel::Nothing),
                server.read_exact(&mut received)
            )
        })
        .await;
        write_result.unwrap();
        read_result.unwrap();
        assert_eq!(received, FRAME);

        // reads return the plaintext
        server.write_all(&MASTER_REPLY).await.unwrap();
        server.flush().await.unwrap();
        let mut buffer = [0; LINK_HEADER_LENGTH];
        let (count, addr) = with_timeout(phys.read(&mut buffer, PhysDecodeLevel::Nothing))
            .await
            .unwrap();
        assert_eq!(
            (count, addr, buffer),
            (MASTER_REPLY.len(), PhysAddr::None, MASTER_REPLY)
        );
    }
}
