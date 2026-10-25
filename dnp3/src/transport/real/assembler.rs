// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::link::header::FrameInfo;
use crate::link::EndpointAddress;
use crate::transport::real::header::Header;
use crate::transport::{Fragment, FragmentAddr, FragmentInfo};

/// Reassembles transport segments into fragments, following the transport reception state table
/// of IEEE 1815-2012 8.2.4 as replaced by TB2013-003.
///
/// Extensions to the table:
/// - a unicast segment that isn't FIR, from a different source or physical address, abandons the
///   series
/// - broadcast segments are accepted only as a single FIR/FIN segment, and others are ignored
/// - every segment is ignored while a completed fragment waits to be taken
pub(crate) struct Assembler {
    state: State,
    buffer: Box<[u8]>,
}

#[derive(Copy, Clone)]
enum State {
    Idle,
    Running(Series),
    /// the first `len` bytes of the buffer hold a fragment waiting to be taken
    Complete {
        info: FragmentInfo,
        len: usize,
    },
}

/// a series of `len` bytes in progress, whose most recent segment `last` carried `last_len` bytes
#[derive(Copy, Clone)]
struct Series {
    info: FrameInfo,
    last: Header,
    last_len: usize,
    len: usize,
}

struct Overflow;

/// Why the assembler discarded a segment or the series in progress
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Discard {
    /// a FIR segment started a new series, dropping the bytes of the one in progress
    NewSeries { len: usize, source: EndpointAddress },
    /// a broadcast segment that isn't both FIR and FIN
    BroadcastSeries { fir: bool, fin: bool },
    /// a segment that isn't FIR, with no series in progress
    NoSeries { source: EndpointAddress },
    /// a segment from a different source or physical address than the series in progress
    ConflictingAddress {
        previous: FrameInfo,
        received: FrameInfo,
    },
    /// a repeat of the previous segment, which is ignored
    Repeat { seq: u8 },
    /// a segment with the previous segment's sequence number but different contents
    DifferentRepeat { seq: u8 },
    /// a segment with an unexpected sequence number
    UnexpectedSeq { expected: u8, received: u8 },
    /// a segment that doesn't fit in the fragment buffer
    Overflow { len: usize },
    /// a segment received while a completed fragment waits to be taken
    Pending,
}

impl Discard {
    /// log the reason, which the reader passes to [`Assembler::assemble`]
    pub(crate) fn log(self) {
        match self {
            Self::Repeat { .. } | Self::Pending => tracing::debug!("transport: {self}"),
            _ => tracing::warn!("transport: {self}"),
        }
    }
}

impl std::fmt::Display for Discard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewSeries { len, source } => {
                write!(f, "received FIR - dropping {len} assembled bytes from {source}")
            }
            Self::BroadcastSeries { fir, fin } => write!(
                f,
                "ignoring broadcast frame with transport header fir: {fir} and fin: {fin}"
            ),
            Self::NoSeries { source } => {
                write!(f, "ignoring non-FIR segment from {source} with no previous FIR")
            }
            Self::ConflictingAddress { previous, received } => write!(
                f,
                "conflicting addresses, previous segment with {previous:?}, but received {received:?}"
            ),
            Self::Repeat { seq } => write!(f, "ignoring repeated segment with seq: {seq}"),
            Self::DifferentRepeat { seq } => write!(
                f,
                "segment with repeated seq: {seq} differs from the previous segment"
            ),
            Self::UnexpectedSeq { expected, received } => {
                write!(f, "expected seq: {expected} but received seq: {received}")
            }
            Self::Overflow { len } => write!(f, "buffer overflow with {len} bytes to write"),
            Self::Pending => f.write_str("ignoring segment while a fragment waits to be read"),
        }
    }
}

impl Assembler {
    pub(crate) fn new(max_fragment_size: usize) -> Self {
        Self {
            state: State::Idle,
            buffer: vec![0; max_fragment_size].into_boxed_slice(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.state = State::Idle;
    }

    pub(crate) fn peek(&self) -> Option<Fragment<'_>> {
        let State::Complete { info, len } = self.state else {
            return None;
        };
        Some(Fragment {
            info,
            data: self.fragment(len),
        })
    }

    pub(crate) fn pop(&mut self) -> Option<Fragment<'_>> {
        let State::Complete { info, len } = self.state else {
            return None;
        };
        self.state = State::Idle;
        Some(Fragment {
            info,
            data: self.fragment(len),
        })
    }

    /// Add a segment, passing the reason to `discard` whenever a segment or the series in
    /// progress is discarded
    pub(crate) fn assemble<F: FnMut(Discard)>(
        &mut self,
        info: FrameInfo,
        header: Header,
        data: &[u8],
        mut discard: F,
    ) {
        let series = match self.state {
            State::Idle => None,
            State::Running(series) => Some(series),
            State::Complete { .. } => {
                discard(Discard::Pending);
                return;
            }
        };

        // FIR always clears the previous assembly
        if header.fir {
            if let Some(series) = series {
                discard(Discard::NewSeries {
                    len: series.len,
                    source: series.info.source,
                });
            }
            self.state = State::Idle;
        }

        if info.broadcast.is_some() {
            if header.fir && header.fin {
                self.append(0, info, header, data, &mut discard);
            } else {
                discard(Discard::BroadcastSeries {
                    fir: header.fir,
                    fin: header.fin,
                });
            }
        } else if header.fir {
            self.append(0, info, header, data, &mut discard);
        } else if let Some(series) = series {
            self.continue_series(series, info, header, data, &mut discard);
        } else {
            discard(Discard::NoSeries {
                source: info.source,
            });
        }
    }

    /// Add a segment that isn't FIR to the series in progress
    fn continue_series<F: FnMut(Discard)>(
        &mut self,
        series: Series,
        info: FrameInfo,
        header: Header,
        data: &[u8],
        discard: &mut F,
    ) {
        let last = series.last;
        if info != series.info {
            discard(Discard::ConflictingAddress {
                previous: series.info,
                received: info,
            });
            self.state = State::Idle;
        } else if header.seq.value() == last.seq.value() {
            let repeat = header.to_u8() == last.to_u8()
                && data.len() == series.last_len
                && self.fragment(series.len).ends_with(data);
            if repeat {
                discard(Discard::Repeat {
                    seq: header.seq.value(),
                });
            } else {
                discard(Discard::DifferentRepeat {
                    seq: header.seq.value(),
                });
                self.state = State::Idle;
            }
        } else if header.seq.value() != last.seq.next() {
            discard(Discard::UnexpectedSeq {
                expected: last.seq.next(),
                received: header.seq.value(),
            });
            self.state = State::Idle;
        } else {
            self.append(series.len, info, header, data, discard);
        }
    }

    /// the first `len` bytes of the buffer
    fn fragment(&self, len: usize) -> &[u8] {
        self.buffer.get(..len).unwrap_or_default()
    }

    /// Append a segment to the `start` bytes already assembled
    fn append<F: FnMut(Discard)>(
        &mut self,
        start: usize,
        info: FrameInfo,
        header: Header,
        data: &[u8],
        discard: &mut F,
    ) {
        let Ok(len) = self.extend(start, data) else {
            discard(Discard::Overflow { len: data.len() });
            self.state = State::Idle;
            return;
        };

        self.state = if !header.fin {
            State::Running(Series {
                info,
                last: header,
                last_len: data.len(),
                len,
            })
        } else if len == 0 {
            // an empty fragment isn't passed to the application layer
            State::Idle
        } else {
            let addr = FragmentAddr {
                link: info.source,
                phys: info.phys_addr,
            };
            State::Complete {
                info: FragmentInfo::new(addr, info.broadcast),
                len,
            }
        };
    }

    /// Copy `data` into the buffer after the first `start` bytes, returning the new length
    fn extend(&mut self, start: usize, data: &[u8]) -> Result<usize, Overflow> {
        let end = start.checked_add(data.len()).ok_or(Overflow)?;
        let dest = self.buffer.get_mut(start..end).ok_or(Overflow)?;
        for (dest, src) in dest.iter_mut().zip(data) {
            *dest = *src;
        }
        Ok(end)
    }
}

/// Tests named after the rows of the transport reception state table as replaced by TB2013-003
#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::header::{BroadcastConfirmMode, FrameType};
    use crate::link::EndpointAddress;
    use crate::phys::PhysAddr;

    const MAX: usize = 8;

    fn from(source: u16) -> FrameInfo {
        FrameInfo::new(
            EndpointAddress::raw(source),
            None,
            FrameType::Data,
            PhysAddr::None,
        )
    }

    fn info() -> FrameInfo {
        from(1)
    }

    fn broadcast() -> FrameInfo {
        FrameInfo::new(
            EndpointAddress::raw(1),
            Some(BroadcastConfirmMode::Mandatory),
            FrameType::Data,
            PhysAddr::None,
        )
    }

    struct Harness {
        assembler: Assembler,
        discards: Vec<Discard>,
    }

    impl Harness {
        fn new() -> Self {
            Self {
                assembler: Assembler::new(MAX),
                discards: Vec::new(),
            }
        }

        /// returns true if the segment completed a fragment
        fn segment(&mut self, fir: bool, fin: bool, seq: u8, data: &[u8]) -> bool {
            let header = crate::link::test_data::segment(fir, fin, seq);
            let discards = &mut self.discards;
            self.assembler
                .assemble(info(), header, data, |d| discards.push(d));
            self.assembler.peek().is_some()
        }

        fn first(&mut self, seq: u8, data: &[u8]) -> bool {
            self.segment(true, false, seq, data)
        }

        fn middle(&mut self, seq: u8, data: &[u8]) -> bool {
            self.segment(false, false, seq, data)
        }

        fn last(&mut self, seq: u8, data: &[u8]) -> bool {
            self.segment(false, true, seq, data)
        }

        fn single(&mut self, seq: u8, data: &[u8]) -> bool {
            self.segment(true, true, seq, data)
        }

        /// takes the discard reasons reported since the last call
        fn discards(&mut self) -> Vec<Discard> {
            std::mem::take(&mut self.discards)
        }

        fn pop(&mut self) -> Option<Vec<u8>> {
            self.assembler.pop().map(|fragment| fragment.data.to_vec())
        }

        /// a fresh single-segment fragment still assembles
        fn assert_recovers(&mut self, seq: u8) {
            assert!(self.single(seq, &[0xEE]));
            assert_eq!(self.pop(), Some(vec![0xEE]));
        }
    }

    #[test]
    fn row_1_idle_discards_a_segment_without_fir() {
        let mut h = Harness::new();
        assert!(!h.middle(0, &[1]));
        assert!(!h.last(1, &[2]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(2);
    }

    #[test]
    fn row_2_empty_single_segment_is_not_passed_up() {
        let mut h = Harness::new();
        assert!(!h.single(0, &[]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(1);
    }

    #[test]
    fn row_3_single_segment_is_passed_up() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1, 2, 3]));
        assert_eq!(h.pop(), Some(vec![1, 2, 3]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn row_3_single_segment_of_exactly_the_maximum_size_is_passed_up() {
        let mut h = Harness::new();
        assert!(h.single(0, &[7; MAX]));
        assert_eq!(h.pop(), Some(vec![7; MAX]));
    }

    #[test]
    fn row_4_single_segment_over_the_maximum_size_is_discarded() {
        let mut h = Harness::new();
        assert!(!h.single(0, &[7; MAX + 1]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(1);
    }

    #[test]
    fn row_5_empty_first_segment_starts_a_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[]));
        assert!(h.last(1, &[1, 2]));
        assert_eq!(h.pop(), Some(vec![1, 2]));
    }

    #[test]
    fn row_6_first_segment_starts_a_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert_eq!(h.pop(), None);
        assert!(h.last(1, &[2]));
        assert_eq!(h.pop(), Some(vec![1, 2]));
    }

    #[test]
    fn row_7_first_segment_over_the_maximum_size_is_discarded() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[7; MAX + 1]));
        // the discarded series can't be continued
        assert!(!h.last(1, &[1]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(2);
    }

    #[test]
    fn row_8_identical_repeat_is_discarded_and_assembly_continues() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2, 3]));
        assert!(!h.middle(1, &[2, 3]));
        assert!(!h.middle(1, &[2, 3]));
        assert_eq!(h.discards(), [Discard::Repeat { seq: 1 }; 2]);
        assert!(h.last(2, &[4]));
        assert_eq!(h.pop(), Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn row_8_identical_repeat_of_an_empty_segment_is_discarded() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[]));
        assert!(!h.middle(1, &[]));
        assert!(h.last(2, &[2]));
        assert_eq!(h.pop(), Some(vec![1, 2]));
    }

    #[test]
    fn row_9_repeat_with_different_bytes_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2, 3]));
        assert!(!h.middle(1, &[2, 4]));
        assert!(!h.last(2, &[5]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(3);
    }

    #[test]
    fn row_9_repeat_of_a_matching_suffix_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2, 3]));
        assert!(!h.middle(1, &[3]));
        assert!(!h.last(2, &[5]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn row_9_repeat_with_a_different_fin_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2]));
        assert!(!h.last(1, &[2]));
        assert!(!h.last(2, &[3]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn row_10_empty_middle_segment_advances_the_sequence() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[]));
        assert!(h.last(2, &[2]));
        assert_eq!(h.pop(), Some(vec![1, 2]));
    }

    #[test]
    fn row_11_segments_of_varying_sizes_append_in_order() {
        let mut h = Harness::new();
        assert!(!h.first(10, &[1, 2, 3]));
        assert!(!h.middle(11, &[4]));
        assert!(!h.middle(12, &[5, 6]));
        assert!(h.last(13, &[7]));
        assert_eq!(h.pop(), Some(vec![1, 2, 3, 4, 5, 6, 7]));
    }

    #[test]
    fn row_11_middle_segment_reaching_exactly_the_maximum_size_appends() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1; MAX - 2]));
        assert!(!h.middle(1, &[2; 2]));
        assert!(h.last(2, &[]));
        assert_eq!(h.pop().map(|x| x.len()), Some(MAX));
    }

    #[test]
    fn row_12_middle_segment_over_the_maximum_size_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1; MAX - 2]));
        assert!(!h.middle(1, &[2; 3]));
        assert!(!h.last(2, &[3]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(3);
    }

    #[test]
    fn row_13_empty_final_segment_of_an_empty_series_is_not_passed_up() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[]));
        assert!(!h.last(1, &[]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(2);
    }

    #[test]
    fn row_14_empty_final_segment_passes_up_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1, 2]));
        assert!(h.last(1, &[]));
        assert_eq!(h.pop(), Some(vec![1, 2]));
    }

    #[test]
    fn row_15_final_segment_reaching_exactly_the_maximum_size_passes_up_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1; MAX - 1]));
        assert!(h.last(1, &[2]));
        assert_eq!(h.pop().map(|x| x.len()), Some(MAX));
    }

    #[test]
    fn row_16_final_segment_over_the_maximum_size_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1; MAX - 1]));
        assert!(!h.last(1, &[2, 3]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(2);
    }

    #[test]
    fn row_17_out_of_order_segment_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(2, &[2]));
        // neither the expected nor the skipped sequence number resumes it
        assert!(!h.last(1, &[3]));
        assert!(!h.last(3, &[3]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(4);
    }

    #[test]
    fn sequence_wraps_from_63_to_0() {
        let mut h = Harness::new();
        assert!(!h.first(62, &[1]));
        assert!(!h.middle(63, &[2]));
        assert!(h.last(0, &[3]));
        assert_eq!(h.pop(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn row_18_empty_first_segment_restarts_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2]));
        assert!(!h.first(5, &[]));
        assert!(h.last(6, &[3]));
        assert_eq!(h.pop(), Some(vec![3]));
    }

    #[test]
    fn row_19_first_segment_restarts_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2]));
        assert!(!h.first(9, &[3]));
        assert!(h.last(10, &[4]));
        assert_eq!(h.pop(), Some(vec![3, 4]));
    }

    #[test]
    fn row_19_repeated_first_segment_restarts_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2]));
        assert!(!h.first(0, &[1]));
        assert!(h.last(1, &[3]));
        assert_eq!(h.pop(), Some(vec![1, 3]));
    }

    #[test]
    fn row_20_first_segment_over_the_maximum_size_discards_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.first(1, &[2; MAX + 1]));
        // neither the earlier nor the new series continues
        assert!(!h.last(1, &[3]));
        assert!(!h.last(2, &[3]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(3);
    }

    #[test]
    fn row_21_empty_single_segment_during_assembly_discards_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.single(4, &[]));
        assert_eq!(h.pop(), None);
        assert!(!h.last(1, &[2]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(5);
    }

    #[test]
    fn row_22_single_segment_during_assembly_replaces_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(h.single(4, &[2]));
        assert_eq!(h.pop(), Some(vec![2]));
    }

    #[test]
    fn row_23_single_segment_over_the_maximum_size_during_assembly_discards_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.single(4, &[2; MAX + 1]));
        assert!(!h.last(1, &[3]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(5);
    }

    /// The assembler against a model written from the reception rules and the documented
    /// extensions, over streams of valid series with one perturbation each
    ///
    /// After every operation the assembler's outcome, discard reasons and waiting fragment must
    /// match the model's. The model is an independent oracle, itself checked against
    /// hand-derived outcomes by `model_follows_the_reception_rules`. The TB2013-003 row tests
    /// above stay as the bulletin's table, and fixed operation lists here pin the documented
    /// extensions and the peek, pop and reset helpers: each gives the outcome of every operation
    /// by hand, which the model must give and the assembler must then match. The property adds
    /// longer sequences and interleavings.
    ///
    /// A case is 1-8 plans, each a valid series of 1-5 segments of up to 4 bytes (or a total forced
    /// to 0, the 8-byte maximum or one past it; the sequence number often wraps), one perturbation
    /// (repeat, changed repeat, drop, skipped sequence number, other source or address, truncate,
    /// insert, interrupt) and a peek, pop or reset tail. Changed repeats are weighted, and most
    /// segments carry data, so that the row 9 cases a repeat must tell apart (the same header and
    /// length with other bytes, a shorter matching suffix) come up in most runs alongside the
    /// empty-segment cases. Known gap: a series conflicting only in broadcast mode is unreachable,
    /// since a broadcast segment never continues a series.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the model adds small lengths, positions and sequence numbers"
    )]
    mod properties {
        use proptest::prelude::*;
        use proptest::sample::Index;

        use super::*;
        use crate::transport::real::sequence::Sequence;

        #[derive(Clone, Debug, PartialEq)]
        struct Segment {
            info: FrameInfo,
            fir: bool,
            fin: bool,
            seq: u8,
            data: Vec<u8>,
        }

        #[derive(Clone, Debug)]
        enum Op {
            Segment(Segment),
            Peek,
            Pop,
            Reset,
        }

        #[derive(Clone, Debug, PartialEq)]
        struct Completed {
            source: EndpointAddress,
            phys: PhysAddr,
            broadcast: Option<BroadcastConfirmMode>,
            data: Vec<u8>,
        }

        impl Completed {
            fn new(fragment: Fragment<'_>) -> Self {
                Self {
                    source: fragment.info.addr.link,
                    phys: fragment.info.addr.phys,
                    broadcast: fragment.info.broadcast,
                    data: fragment.data.to_vec(),
                }
            }
        }

        #[derive(Debug, PartialEq)]
        enum Outcome {
            /// whether a fragment is complete, and why anything was discarded
            Assembled(bool, Vec<Discard>),
            Peeked(Option<Completed>),
            Popped(Option<Completed>),
            Reset,
        }

        enum State {
            Idle,
            /// the bytes received so far, and the segment that added to them last
            Series {
                bytes: Vec<u8>,
                last: Segment,
            },
            Completed(Completed),
        }

        struct Model {
            state: State,
        }

        impl Model {
            fn new() -> Self {
                Self { state: State::Idle }
            }

            fn apply(&mut self, op: &Op) -> Outcome {
                match op {
                    Op::Segment(segment) => {
                        let discards = self.segment(segment);
                        let complete = matches!(self.state, State::Completed(_));
                        Outcome::Assembled(complete, discards)
                    }
                    Op::Peek => Outcome::Peeked(self.completed()),
                    Op::Pop => {
                        let completed = self.completed();
                        if completed.is_some() {
                            self.state = State::Idle;
                        }
                        Outcome::Popped(completed)
                    }
                    Op::Reset => {
                        self.state = State::Idle;
                        Outcome::Reset
                    }
                }
            }

            fn completed(&self) -> Option<Completed> {
                match &self.state {
                    State::Completed(x) => Some(x.clone()),
                    State::Idle | State::Series { .. } => None,
                }
            }

            /// receive a segment, returning why anything was discarded
            fn segment(&mut self, segment: &Segment) -> Vec<Discard> {
                let series = match std::mem::replace(&mut self.state, State::Idle) {
                    // nothing is received while a completed fragment waits to be taken
                    State::Completed(x) => {
                        self.state = State::Completed(x);
                        return vec![Discard::Pending];
                    }
                    State::Idle => None,
                    State::Series { bytes, last } => Some((bytes, last)),
                };

                let mut discards = Vec::new();
                if segment.fir {
                    if let Some((bytes, last)) = &series {
                        discards.push(Discard::NewSeries {
                            len: bytes.len(),
                            source: last.info.source,
                        });
                    }
                }

                let broadcast = segment.info.broadcast.is_some();
                match series {
                    _ if broadcast && segment.fir && segment.fin => {
                        self.append(Vec::new(), segment, &mut discards)
                    }
                    series if broadcast => {
                        discards.push(Discard::BroadcastSeries {
                            fir: segment.fir,
                            fin: segment.fin,
                        });
                        // a broadcast that isn't FIR leaves the series in progress
                        if let (false, Some((bytes, last))) = (segment.fir, series) {
                            self.state = State::Series { bytes, last };
                        }
                    }
                    _ if segment.fir => self.append(Vec::new(), segment, &mut discards),
                    None => discards.push(Discard::NoSeries {
                        source: segment.info.source,
                    }),
                    Some((bytes, last)) => self.next(bytes, last, segment, &mut discards),
                }
                discards
            }

            /// receive a segment that isn't FIR during a series
            fn next(
                &mut self,
                bytes: Vec<u8>,
                last: Segment,
                segment: &Segment,
                discards: &mut Vec<Discard>,
            ) {
                let expected = (last.seq + 1) % 64;
                if segment.info != last.info {
                    discards.push(Discard::ConflictingAddress {
                        previous: last.info,
                        received: segment.info,
                    });
                } else if segment.seq == last.seq && *segment == last {
                    discards.push(Discard::Repeat { seq: segment.seq });
                    self.state = State::Series { bytes, last };
                } else if segment.seq == last.seq {
                    discards.push(Discard::DifferentRepeat { seq: segment.seq });
                } else if segment.seq != expected {
                    discards.push(Discard::UnexpectedSeq {
                        expected,
                        received: segment.seq,
                    });
                } else {
                    self.append(bytes, segment, discards);
                }
            }

            fn append(
                &mut self,
                mut bytes: Vec<u8>,
                segment: &Segment,
                discards: &mut Vec<Discard>,
            ) {
                if bytes.len() + segment.data.len() > MAX {
                    discards.push(Discard::Overflow {
                        len: segment.data.len(),
                    });
                    return;
                }
                bytes.extend_from_slice(&segment.data);
                self.state = if !segment.fin {
                    State::Series {
                        bytes,
                        last: segment.clone(),
                    }
                } else if bytes.is_empty() {
                    State::Idle
                } else {
                    State::Completed(Completed {
                        source: segment.info.source,
                        phys: segment.info.phys_addr,
                        broadcast: segment.info.broadcast,
                        data: bytes,
                    })
                };
            }
        }

        fn apply(assembler: &mut Assembler, op: &Op) -> Outcome {
            match op {
                Op::Segment(x) => {
                    let header = Header::new(x.fin, x.fir, Sequence::new(x.seq));
                    let mut discards = Vec::new();
                    assembler.assemble(x.info, header, &x.data, |d| discards.push(d));
                    Outcome::Assembled(assembler.peek().is_some(), discards)
                }
                Op::Peek => Outcome::Peeked(assembler.peek().map(Completed::new)),
                Op::Pop => Outcome::Popped(assembler.pop().map(Completed::new)),
                Op::Reset => {
                    assembler.reset();
                    Outcome::Reset
                }
            }
        }

        /// the assembler and the model agree after every operation, including on the fragment
        /// waiting to be taken
        fn check(ops: &[Op]) -> Result<(), TestCaseError> {
            let mut assembler = Assembler::new(MAX);
            let mut model = Model::new();
            for (i, op) in ops.iter().enumerate() {
                prop_assert_eq!(
                    apply(&mut assembler, op),
                    model.apply(op),
                    "operation {}: {:?}",
                    i,
                    op
                );
                prop_assert_eq!(
                    assembler.peek().map(Completed::new),
                    model.completed(),
                    "after operation {}",
                    i
                );
            }
            Ok(())
        }

        #[derive(Clone, Debug)]
        enum Change {
            Byte,
            Longer,
            Suffix,
            Fin,
            Fir,
        }

        impl Change {
            fn apply(&self, segment: &mut Segment) {
                match self {
                    Self::Byte => match segment.data.first_mut() {
                        Some(x) => *x = !*x,
                        None => segment.data.push(0),
                    },
                    Self::Longer => segment.data.push(0),
                    Self::Suffix if segment.data.is_empty() => segment.data.push(0),
                    Self::Suffix => {
                        segment.data.remove(0);
                    }
                    Self::Fin => segment.fin = !segment.fin,
                    Self::Fir => segment.fir = !segment.fir,
                }
            }
        }

        #[derive(Clone, Debug)]
        enum Perturbation {
            None,
            /// send a segment twice
            Repeat(Index),
            /// send a segment again, changed
            ChangedRepeat(Index, Change),
            /// leave a segment out
            Drop(Index),
            /// increment the sequence numbers from a segment on
            SkipSeq(Index),
            /// send a segment from another source, physical address or broadcast
            OtherInfo(Index, FrameInfo),
            /// stop before a segment
            Truncate(Index),
            /// send another segment before a segment, or after the last
            Insert(Index, Segment),
            /// peek, pop or reset before a segment, or after the last
            Interrupt(Index, Op),
        }

        /// a valid series, perturbed, followed by peeks, pops or a reset
        #[derive(Clone, Debug)]
        struct Plan {
            info: FrameInfo,
            first_seq: u8,
            sizes: Vec<usize>,
            /// the total length, if the sizes are adjusted to it
            total: Option<usize>,
            seed: u8,
            perturbation: Perturbation,
            tail: Vec<Op>,
        }

        impl Plan {
            fn series(&self) -> Vec<Segment> {
                let mut sizes = self.sizes.clone();
                if let Some(total) = self.total {
                    let mut remaining = total;
                    let last = sizes.len() - 1;
                    for (i, size) in sizes.iter_mut().enumerate() {
                        *size = if i == last {
                            remaining
                        } else {
                            remaining.min(*size)
                        };
                        remaining -= *size;
                    }
                }

                let mut seq = self.first_seq;
                let mut byte = self.seed;
                let last = sizes.len() - 1;
                let mut segments = Vec::new();
                for (i, size) in sizes.into_iter().enumerate() {
                    let data = (0..size)
                        .map(|_| {
                            byte = byte.wrapping_add(1);
                            byte
                        })
                        .collect();
                    segments.push(Segment {
                        info: self.info,
                        fir: i == 0,
                        fin: i == last,
                        seq,
                        data,
                    });
                    seq = (seq + 1) % 64;
                }
                segments
            }

            fn ops(&self) -> Vec<Op> {
                let mut segments = self.series();
                let len = segments.len();
                match &self.perturbation {
                    Perturbation::None | Perturbation::Interrupt(..) => {}
                    Perturbation::Repeat(i) => {
                        let i = i.index(len);
                        segments.insert(i + 1, segments[i].clone());
                    }
                    Perturbation::ChangedRepeat(i, change) => {
                        let i = i.index(len);
                        let mut segment = segments[i].clone();
                        change.apply(&mut segment);
                        segments.insert(i + 1, segment);
                    }
                    Perturbation::Drop(i) => {
                        segments.remove(i.index(len));
                    }
                    Perturbation::SkipSeq(i) => {
                        for segment in &mut segments[i.index(len)..] {
                            segment.seq = (segment.seq + 1) % 64;
                        }
                    }
                    Perturbation::OtherInfo(i, info) => segments[i.index(len)].info = *info,
                    Perturbation::Truncate(i) => segments.truncate(i.index(len)),
                    Perturbation::Insert(i, segment) => {
                        segments.insert(i.index(len + 1), segment.clone())
                    }
                }
                let mut ops: Vec<Op> = segments.into_iter().map(Op::Segment).collect();
                if let Perturbation::Interrupt(i, op) = &self.perturbation {
                    ops.insert(i.index(ops.len() + 1), op.clone());
                }
                ops.extend(self.tail.iter().cloned());
                ops
            }
        }

        fn infos() -> Vec<FrameInfo> {
            let mut optional = broadcast();
            optional.broadcast = Some(BroadcastConfirmMode::Optional);
            vec![info(), from(2), udp(), broadcast(), optional]
        }

        fn frame_info() -> impl Strategy<Value = FrameInfo> {
            prop_oneof![
                3 => Just(info()),
                2 => prop::sample::select(infos()),
            ]
        }

        fn seq() -> impl Strategy<Value = u8> {
            // the sequence number wraps within the series
            prop_oneof![0u8..64, 60u8..64]
        }

        fn interrupt() -> impl Strategy<Value = Op> {
            prop_oneof![Just(Op::Peek), Just(Op::Pop), Just(Op::Reset)]
        }

        fn change() -> impl Strategy<Value = Change> {
            prop_oneof![
                2 => Just(Change::Byte),
                1 => Just(Change::Longer),
                2 => Just(Change::Suffix),
                1 => Just(Change::Fin),
                1 => Just(Change::Fir),
            ]
        }

        fn segment() -> impl Strategy<Value = Segment> {
            (
                prop::sample::select(infos()),
                any::<bool>(),
                any::<bool>(),
                seq(),
                prop::collection::vec(any::<u8>(), 0..=3),
            )
                .prop_map(|(info, fir, fin, seq, data)| Segment {
                    info,
                    fir,
                    fin,
                    seq,
                    data,
                })
        }

        fn perturbation() -> impl Strategy<Value = Perturbation> {
            let i = any::<Index>;
            prop_oneof![
                2 => Just(Perturbation::None),
                2 => i().prop_map(Perturbation::Repeat),
                6 => (i(), change()).prop_map(|(i, x)| Perturbation::ChangedRepeat(i, x)),
                1 => i().prop_map(Perturbation::Drop),
                1 => i().prop_map(Perturbation::SkipSeq),
                2 => (i(), prop::sample::select(infos()))
                    .prop_map(|(i, x)| Perturbation::OtherInfo(i, x)),
                2 => i().prop_map(Perturbation::Truncate),
                2 => (i(), segment()).prop_map(|(i, x)| Perturbation::Insert(i, x)),
                2 => (i(), interrupt()).prop_map(|(i, x)| Perturbation::Interrupt(i, x)),
            ]
        }

        fn tail() -> impl Strategy<Value = Vec<Op>> {
            // nothing taken leaves a completed fragment waiting
            prop_oneof![
                3 => Just(Vec::new()),
                2 => Just(vec![Op::Pop]),
                1 => Just(vec![Op::Peek, Op::Pop]),
                1 => Just(vec![Op::Peek]),
                1 => Just(vec![Op::Reset]),
            ]
        }

        fn plan() -> impl Strategy<Value = Plan> {
            (
                frame_info(),
                seq(),
                // a changed repeat of an empty segment is a length change
                prop::collection::vec(prop_oneof![1 => Just(0), 3 => 0usize..=4], 1..=5),
                prop_oneof![
                    4 => Just(None),
                    1 => Just(Some(0)),
                    2 => Just(Some(MAX)),
                    2 => Just(Some(MAX + 1)),
                ],
                any::<u8>(),
                perturbation(),
                tail(),
            )
                .prop_map(
                    |(info, first_seq, sizes, total, seed, perturbation, tail)| Plan {
                        info,
                        first_seq,
                        sizes,
                        total,
                        seed,
                        perturbation,
                        tail,
                    },
                )
        }

        fn seg(info: FrameInfo, fir: bool, fin: bool, seq: u8, data: &[u8]) -> Op {
            Op::Segment(Segment {
                info,
                fir,
                fin,
                seq,
                data: data.to_vec(),
            })
        }

        fn completed(info: FrameInfo, data: &[u8]) -> Option<Completed> {
            Some(Completed {
                source: info.source,
                phys: info.phys_addr,
                broadcast: info.broadcast,
                data: data.to_vec(),
            })
        }

        /// hand-derived outcomes where a plausible assembler goes wrong, so the model can't
        /// accept them
        #[test]
        fn model_follows_the_reception_rules() {
            let a = info();
            let b = from(2);
            let assembled = Outcome::Assembled;
            let cases = [
                // a completed fragment is kept while later segments are ignored
                (seg(a, true, true, 0, &[1, 2]), assembled(true, vec![])),
                (
                    seg(b, true, true, 1, &[3]),
                    assembled(true, vec![Discard::Pending]),
                ),
                (Op::Peek, Outcome::Peeked(completed(a, &[1, 2]))),
                (Op::Pop, Outcome::Popped(completed(a, &[1, 2]))),
                (Op::Pop, Outcome::Popped(None)),
                // a repeat of a matching suffix isn't a repeat
                (seg(a, true, false, 62, &[4]), assembled(false, vec![])),
                (seg(a, false, false, 63, &[5, 6]), assembled(false, vec![])),
                (
                    seg(a, false, false, 63, &[6]),
                    assembled(false, vec![Discard::DifferentRepeat { seq: 63 }]),
                ),
                (
                    seg(a, false, true, 0, &[7]),
                    assembled(false, vec![Discard::NoSeries { source: a.source }]),
                ),
                // the sequence number wraps, and an identical repeat is ignored
                (seg(b, true, false, 63, &[8]), assembled(false, vec![])),
                (
                    seg(b, true, false, 63, &[8]),
                    assembled(
                        false,
                        vec![Discard::NewSeries {
                            len: 1,
                            source: b.source,
                        }],
                    ),
                ),
                (seg(b, false, false, 0, &[]), assembled(false, vec![])),
                (
                    seg(b, false, false, 0, &[]),
                    assembled(false, vec![Discard::Repeat { seq: 0 }]),
                ),
                // a broadcast continuation leaves the series, and the series fills the buffer
                (
                    seg(broadcast(), false, true, 1, &[9]),
                    assembled(
                        false,
                        vec![Discard::BroadcastSeries {
                            fir: false,
                            fin: true,
                        }],
                    ),
                ),
                (
                    seg(b, false, true, 1, &[1; MAX - 1]),
                    assembled(true, vec![]),
                ),
                (Op::Reset, Outcome::Reset),
                (Op::Peek, Outcome::Peeked(None)),
            ];
            follows(&cases);
        }

        /// the model gives each hand-derived outcome, and the assembler agrees with the model
        /// after every operation, so the assembler gives them too
        fn follows(cases: &[(Op, Outcome)]) {
            let mut model = Model::new();
            for (op, outcome) in cases {
                assert_eq!(&model.apply(op), outcome, "{op:?}");
            }
            let ops: Vec<Op> = cases.iter().map(|(op, _)| op.clone()).collect();
            check(&ops).unwrap();
        }

        // Fixed operation lists for the documented extensions and the peek, pop and reset
        // helpers, each outcome derived by hand

        const SOURCE: EndpointAddress = EndpointAddress::raw(1);
        /// a segment that leaves no fragment waiting and discards nothing
        const INCOMPLETE: Outcome = Outcome::Assembled(false, Vec::new());
        /// a segment that completes a fragment and discards nothing
        const COMPLETE: Outcome = Outcome::Assembled(true, Vec::new());
        const POPPED_NOTHING: Outcome = Outcome::Popped(None);

        fn first(seq: u8, data: &[u8]) -> Op {
            seg(info(), true, false, seq, data)
        }

        fn middle(seq: u8, data: &[u8]) -> Op {
            seg(info(), false, false, seq, data)
        }

        fn last(seq: u8, data: &[u8]) -> Op {
            seg(info(), false, true, seq, data)
        }

        fn single(seq: u8, data: &[u8]) -> Op {
            seg(info(), true, true, seq, data)
        }

        /// a segment that completes nothing and is discarded, or discards the series
        fn discarded(reason: Discard) -> Outcome {
            Outcome::Assembled(false, vec![reason])
        }

        fn no_series() -> Outcome {
            discarded(Discard::NoSeries { source: SOURCE })
        }

        fn new_series(len: usize) -> Discard {
            Discard::NewSeries {
                len,
                source: SOURCE,
            }
        }

        fn conflict(received: FrameInfo) -> Outcome {
            discarded(Discard::ConflictingAddress {
                previous: info(),
                received,
            })
        }

        fn broadcast_series(fir: bool, fin: bool) -> Discard {
            Discard::BroadcastSeries { fir, fin }
        }

        fn unexpected(expected: u8, received: u8) -> Outcome {
            discarded(Discard::UnexpectedSeq { expected, received })
        }

        fn popped(info: FrameInfo, data: &[u8]) -> Outcome {
            Outcome::Popped(completed(info, data))
        }

        fn udp() -> FrameInfo {
            let mut udp = info();
            udp.phys_addr = PhysAddr::Udp("127.0.0.1:20000".parse().unwrap());
            udp
        }

        #[test]
        fn repeated_final_segment_after_the_fragment_is_taken_is_discarded() {
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (last(1, &[2]), COMPLETE),
                (Op::Pop, popped(info(), &[1, 2])),
                (last(1, &[2]), no_series()),
                (Op::Pop, POPPED_NOTHING),
            ]);
        }

        #[test]
        fn repeated_single_segment_is_passed_up_again() {
            follows(&[
                (single(0, &[1]), COMPLETE),
                (Op::Pop, popped(info(), &[1])),
                (single(0, &[1]), COMPLETE),
                (Op::Pop, popped(info(), &[1])),
            ]);
        }

        #[test]
        fn segment_from_another_source_abandons_the_series() {
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (seg(from(2), false, true, 1, &[2]), conflict(from(2))),
                (last(1, &[2]), no_series()),
                (Op::Pop, POPPED_NOTHING),
                (single(2, &[0xEE]), COMPLETE),
                (Op::Pop, popped(info(), &[0xEE])),
            ]);
        }

        #[test]
        fn identical_repeat_from_another_source_abandons_the_series() {
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (middle(1, &[2]), INCOMPLETE),
                (seg(from(2), false, false, 1, &[2]), conflict(from(2))),
                (last(2, &[3]), no_series()),
                (Op::Pop, POPPED_NOTHING),
            ]);
        }

        #[test]
        fn segment_from_another_physical_address_abandons_the_series() {
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (seg(udp(), false, true, 1, &[2]), conflict(udp())),
                (last(1, &[3]), no_series()),
                (Op::Pop, POPPED_NOTHING),
            ]);
        }

        #[test]
        fn first_segment_from_another_source_starts_a_new_series() {
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (seg(from(2), true, false, 7, &[2]), discarded(new_series(1))),
                (seg(from(2), false, true, 8, &[3]), COMPLETE),
                (Op::Pop, popped(from(2), &[2, 3])),
            ]);
        }

        #[test]
        fn broadcast_single_segment_is_passed_up_with_its_mode() {
            follows(&[
                (seg(broadcast(), true, true, 0, &[1]), COMPLETE),
                (Op::Pop, popped(broadcast(), &[1])),
            ]);
        }

        #[test]
        fn broadcast_series_is_ignored() {
            let fir_only = broadcast_series(true, false);
            let fin_only = broadcast_series(false, true);
            follows(&[
                (seg(broadcast(), true, false, 0, &[1]), discarded(fir_only)),
                (seg(broadcast(), false, true, 1, &[2]), discarded(fin_only)),
                (Op::Pop, POPPED_NOTHING),
                (single(2, &[0xEE]), COMPLETE),
                (Op::Pop, popped(info(), &[0xEE])),
            ]);
        }

        #[test]
        fn broadcast_first_segment_abandons_a_series_in_progress() {
            let fir_only = broadcast_series(true, false);
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (
                    seg(broadcast(), true, false, 5, &[9]),
                    Outcome::Assembled(false, vec![new_series(1), fir_only]),
                ),
                (last(1, &[2]), no_series()),
                (Op::Pop, POPPED_NOTHING),
            ]);
        }

        #[test]
        fn broadcast_continuation_leaves_a_series_in_progress() {
            let fin_only = broadcast_series(false, true);
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (seg(broadcast(), false, true, 1, &[9]), discarded(fin_only)),
                (last(1, &[2]), COMPLETE),
                (Op::Pop, popped(info(), &[1, 2])),
            ]);
        }

        #[test]
        fn broadcast_single_segment_replaces_a_series_in_progress() {
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (
                    seg(broadcast(), true, true, 5, &[9]),
                    Outcome::Assembled(true, vec![new_series(1)]),
                ),
                (Op::Pop, popped(broadcast(), &[9])),
                (last(1, &[2]), no_series()),
            ]);
        }

        #[test]
        fn peek_leaves_the_fragment_and_pop_takes_it_once() {
            follows(&[
                (single(0, &[1]), COMPLETE),
                (Op::Peek, Outcome::Peeked(completed(info(), &[1]))),
                (Op::Peek, Outcome::Peeked(completed(info(), &[1]))),
                (Op::Pop, popped(info(), &[1])),
                (Op::Peek, Outcome::Peeked(None)),
                (Op::Pop, POPPED_NOTHING),
            ]);
        }

        #[test]
        fn reset_discards_a_series_in_progress() {
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (Op::Reset, Outcome::Reset),
                (last(1, &[2]), no_series()),
                (Op::Pop, POPPED_NOTHING),
                (single(2, &[0xEE]), COMPLETE),
                (Op::Pop, popped(info(), &[0xEE])),
            ]);
        }

        #[test]
        fn new_series_reports_the_bytes_it_drops() {
            follows(&[
                (first(0, &[1, 2]), INCOMPLETE),
                (middle(1, &[3]), INCOMPLETE),
                (first(5, &[4]), discarded(new_series(3))),
                (last(6, &[5]), COMPLETE),
                (Op::Pop, popped(info(), &[4, 5])),
            ]);
        }

        #[test]
        fn new_series_reports_an_empty_series_it_drops() {
            follows(&[
                (first(0, &[]), INCOMPLETE),
                (first(1, &[1]), discarded(new_series(0))),
            ]);
        }

        #[test]
        fn segments_are_ignored_while_a_fragment_waits_to_be_taken() {
            let pending = || Outcome::Assembled(true, vec![Discard::Pending]);
            follows(&[
                (single(0, &[1, 2]), COMPLETE),
                (first(1, &[3]), pending()),
                (single(2, &[4]), pending()),
                (middle(3, &[5]), pending()),
                (last(4, &[6]), pending()),
                (first(5, &[7; MAX + 1]), pending()),
                (seg(broadcast(), true, true, 6, &[8]), pending()),
                (seg(broadcast(), false, true, 7, &[9]), pending()),
                (Op::Pop, popped(info(), &[1, 2])),
                (single(8, &[0xEE]), COMPLETE),
                (Op::Pop, popped(info(), &[0xEE])),
            ]);
        }

        #[test]
        fn series_after_a_taken_fragment_starts_empty() {
            follows(&[
                (single(0, &[1; MAX]), COMPLETE),
                (Op::Pop, popped(info(), &[1; MAX])),
                (first(1, &[2; MAX - 1]), INCOMPLETE),
                (last(2, &[3]), COMPLETE),
                (Op::Pop, popped(info(), &[2, 2, 2, 2, 2, 2, 2, 3])),
            ]);
        }

        #[test]
        fn series_after_a_reset_starts_empty() {
            follows(&[
                (first(0, &[1; MAX - 1]), INCOMPLETE),
                (Op::Reset, Outcome::Reset),
                (first(1, &[2; MAX - 1]), INCOMPLETE),
                (last(2, &[3]), COMPLETE),
                (Op::Pop, popped(info(), &[2, 2, 2, 2, 2, 2, 2, 3])),
            ]);
        }

        #[test]
        fn reset_after_a_completed_fragment_recovers() {
            follows(&[
                (single(0, &[1; MAX]), COMPLETE),
                (Op::Reset, Outcome::Reset),
                (Op::Peek, Outcome::Peeked(None)),
                (Op::Pop, POPPED_NOTHING),
                (single(1, &[0xEE]), COMPLETE),
                (Op::Pop, popped(info(), &[0xEE])),
            ]);
        }

        #[test]
        fn series_after_an_abandoned_series_starts_empty() {
            follows(&[
                (first(0, &[1; MAX - 1]), INCOMPLETE),
                (middle(2, &[2]), unexpected(1, 2)),
                (first(3, &[3; MAX]), INCOMPLETE),
                (last(4, &[]), COMPLETE),
                (Op::Pop, popped(info(), &[3; MAX])),
            ]);
        }

        #[test]
        fn segment_discards_report_their_reasons() {
            let fir_only = broadcast_series(true, false);
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (middle(1, &[2, 3]), INCOMPLETE),
                (middle(1, &[2, 3]), discarded(Discard::Repeat { seq: 1 })),
                (
                    middle(1, &[2, 4]),
                    discarded(Discard::DifferentRepeat { seq: 1 }),
                ),
                (first(10, &[1]), INCOMPLETE),
                (middle(12, &[2]), unexpected(11, 12)),
                (first(20, &[1; MAX - 1]), INCOMPLETE),
                (middle(21, &[2, 3]), discarded(Discard::Overflow { len: 2 })),
                (first(30, &[1]), INCOMPLETE),
                (seg(from(2), false, true, 31, &[2]), conflict(from(2))),
                (seg(broadcast(), true, false, 40, &[1]), discarded(fir_only)),
                (single(41, &[0xEE]), COMPLETE),
                (Op::Pop, popped(info(), &[0xEE])),
            ]);
        }

        #[test]
        fn repeat_of_the_first_segment_without_fir_abandons_the_series() {
            follows(&[
                (first(0, &[1]), INCOMPLETE),
                (
                    middle(0, &[1]),
                    discarded(Discard::DifferentRepeat { seq: 0 }),
                ),
                (last(1, &[2]), no_series()),
                (Op::Pop, POPPED_NOTHING),
            ]);
        }

        proptest! {
            #![proptest_config(crate::util::test_support::proptest_config(128))]

            #[test]
            fn assembler_follows_the_model(plans in prop::collection::vec(plan(), 1..=8)) {
                let ops: Vec<Op> = plans.iter().flat_map(Plan::ops).collect();
                check(&ops)?;
            }
        }
    }
}
