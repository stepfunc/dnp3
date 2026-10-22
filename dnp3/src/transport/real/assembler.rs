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

pub(crate) enum AssemblyState {
    Complete,
    ReadMore,
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
            Self::Repeat { .. } => tracing::debug!("transport: {self}"),
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
    ) -> AssemblyState {
        let series = match self.state {
            State::Idle => None,
            State::Running(series) => Some(series),
            State::Complete { .. } => {
                discard(Discard::Pending);
                return AssemblyState::Complete;
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

        self.assembly_state()
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

    fn assembly_state(&self) -> AssemblyState {
        match self.state {
            State::Complete { .. } => AssemblyState::Complete,
            State::Idle | State::Running(_) => AssemblyState::ReadMore,
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
    use crate::util::phys::PhysAddr;

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
        fn segment(&mut self, info: FrameInfo, fir: bool, fin: bool, seq: u8, data: &[u8]) -> bool {
            let header = crate::link::test_data::segment(fir, fin, seq);
            let discards = &mut self.discards;
            matches!(
                self.assembler
                    .assemble(info, header, data, |d| discards.push(d)),
                AssemblyState::Complete
            )
        }

        fn first(&mut self, seq: u8, data: &[u8]) -> bool {
            self.segment(info(), true, false, seq, data)
        }

        fn middle(&mut self, seq: u8, data: &[u8]) -> bool {
            self.segment(info(), false, false, seq, data)
        }

        fn last(&mut self, seq: u8, data: &[u8]) -> bool {
            self.segment(info(), false, true, seq, data)
        }

        fn single(&mut self, seq: u8, data: &[u8]) -> bool {
            self.segment(info(), true, true, seq, data)
        }

        fn peek(&self) -> Option<Vec<u8>> {
            self.assembler.peek().map(|fragment| fragment.data.to_vec())
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

    #[test]
    fn repeated_final_segment_after_the_fragment_is_taken_is_discarded() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(h.last(1, &[2]));
        assert_eq!(h.pop(), Some(vec![1, 2]));
        assert!(!h.last(1, &[2]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn repeated_single_segment_is_passed_up_again() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1]));
        assert_eq!(h.pop(), Some(vec![1]));
        assert!(h.single(0, &[1]));
        assert_eq!(h.pop(), Some(vec![1]));
    }

    #[test]
    fn segment_from_another_source_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.segment(from(2), false, true, 1, &[2]));
        assert!(!h.last(1, &[2]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(2);
    }

    #[test]
    fn identical_repeat_from_another_source_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2]));
        assert!(!h.segment(from(2), false, false, 1, &[2]));
        assert!(!h.last(2, &[3]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn segment_from_another_physical_address_abandons_the_series() {
        let mut h = Harness::new();
        let mut other = info();
        other.phys_addr = PhysAddr::Udp("127.0.0.1:20000".parse().unwrap());
        assert!(!h.first(0, &[1]));
        assert!(!h.segment(other, false, true, 1, &[2]));
        assert!(!h.last(1, &[3]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn first_segment_from_another_source_starts_a_new_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.segment(from(2), true, false, 7, &[2]));
        assert!(h.segment(from(2), false, true, 8, &[3]));
        let fragment = h.assembler.pop().unwrap();
        assert_eq!(fragment.info.addr.link, EndpointAddress::raw(2));
        assert_eq!(fragment.data, &[2, 3]);
    }

    #[test]
    fn broadcast_single_segment_is_passed_up_with_its_mode() {
        let mut h = Harness::new();
        assert!(h.segment(broadcast(), true, true, 0, &[1]));
        let fragment = h.assembler.pop().unwrap();
        assert_eq!(
            fragment.info.broadcast,
            Some(BroadcastConfirmMode::Mandatory)
        );
        assert_eq!(fragment.data, &[1]);
    }

    #[test]
    fn broadcast_series_is_ignored() {
        let mut h = Harness::new();
        assert!(!h.segment(broadcast(), true, false, 0, &[1]));
        assert!(!h.segment(broadcast(), false, true, 1, &[2]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(2);
    }

    #[test]
    fn broadcast_first_segment_abandons_a_series_in_progress() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.segment(broadcast(), true, false, 5, &[9]));
        assert!(!h.last(1, &[2]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn broadcast_continuation_leaves_a_series_in_progress() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.segment(broadcast(), false, true, 1, &[9]));
        assert!(h.last(1, &[2]));
        assert_eq!(h.pop(), Some(vec![1, 2]));
    }

    #[test]
    fn broadcast_single_segment_replaces_a_series_in_progress() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(h.segment(broadcast(), true, true, 5, &[9]));
        assert_eq!(h.pop(), Some(vec![9]));
        assert!(!h.last(1, &[2]));
    }

    #[test]
    fn peek_leaves_the_fragment_and_pop_takes_it_once() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1]));
        assert_eq!(h.assembler.peek().map(|f| f.data.to_vec()), Some(vec![1]));
        assert_eq!(h.assembler.peek().map(|f| f.data.to_vec()), Some(vec![1]));
        assert_eq!(h.pop(), Some(vec![1]));
        assert!(h.assembler.peek().is_none());
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn reset_discards_a_series_in_progress() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        h.assembler.reset();
        assert!(!h.last(1, &[2]));
        assert_eq!(h.pop(), None);
        h.assert_recovers(2);
    }

    #[test]
    fn new_series_reports_the_bytes_it_drops() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1, 2]));
        assert!(!h.middle(1, &[3]));
        assert!(h.discards().is_empty());
        assert!(!h.first(5, &[4]));
        assert_eq!(
            h.discards(),
            [Discard::NewSeries {
                len: 3,
                source: EndpointAddress::raw(1)
            }]
        );
        assert!(h.last(6, &[5]));
        assert_eq!(h.pop(), Some(vec![4, 5]));
        assert!(h.discards().is_empty());
    }

    #[test]
    fn new_series_reports_an_empty_series_it_drops() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[]));
        assert!(!h.first(1, &[1]));
        assert_eq!(
            h.discards(),
            [Discard::NewSeries {
                len: 0,
                source: EndpointAddress::raw(1)
            }]
        );
    }

    #[test]
    fn segments_are_ignored_while_a_fragment_waits_to_be_taken() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1, 2]));
        assert!(h.first(1, &[3]));
        assert!(h.single(2, &[4]));
        assert!(h.middle(3, &[5]));
        assert!(h.last(4, &[6]));
        assert!(h.first(5, &[7; MAX + 1]));
        assert!(h.segment(broadcast(), true, true, 6, &[8]));
        assert!(h.segment(broadcast(), false, true, 7, &[9]));
        assert_eq!(h.discards(), [Discard::Pending; 7]);
        assert_eq!(h.pop(), Some(vec![1, 2]));
        h.assert_recovers(8);
    }

    #[test]
    fn series_after_a_taken_fragment_starts_empty() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1; MAX]));
        assert_eq!(h.pop(), Some(vec![1; MAX]));
        assert!(!h.first(1, &[2; MAX - 1]));
        assert!(h.last(2, &[3]));
        let mut expected = vec![2; MAX - 1];
        expected.push(3);
        assert_eq!(h.pop(), Some(expected));
        assert!(h.discards().is_empty());
    }

    #[test]
    fn series_after_a_reset_starts_empty() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1; MAX - 1]));
        h.assembler.reset();
        assert!(!h.first(1, &[2; MAX - 1]));
        assert!(h.discards().is_empty());
        assert!(h.last(2, &[3]));
        assert_eq!(h.pop().map(|x| x.len()), Some(MAX));
    }

    #[test]
    fn reset_after_a_completed_fragment_recovers() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1; MAX]));
        h.assembler.reset();
        assert_eq!(h.peek(), None);
        assert_eq!(h.pop(), None);
        h.assert_recovers(1);
    }

    #[test]
    fn series_after_an_abandoned_series_starts_empty() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1; MAX - 1]));
        assert!(!h.middle(2, &[2]));
        assert!(!h.first(3, &[3; MAX]));
        assert!(h.last(4, &[]));
        assert_eq!(h.pop(), Some(vec![3; MAX]));
    }

    #[test]
    fn segment_discards_report_their_reasons() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2, 3]));
        assert!(!h.middle(1, &[2, 3]));
        assert_eq!(h.discards(), [Discard::Repeat { seq: 1 }]);
        assert!(!h.middle(1, &[2, 4]));
        assert_eq!(h.discards(), [Discard::DifferentRepeat { seq: 1 }]);

        assert!(!h.first(10, &[1]));
        assert!(!h.middle(12, &[2]));
        assert_eq!(
            h.discards(),
            [Discard::UnexpectedSeq {
                expected: 11,
                received: 12
            }]
        );

        assert!(!h.first(20, &[1; MAX - 1]));
        assert!(!h.middle(21, &[2, 3]));
        assert_eq!(h.discards(), [Discard::Overflow { len: 2 }]);

        assert!(!h.first(30, &[1]));
        assert!(!h.segment(from(2), false, true, 31, &[2]));
        assert_eq!(
            h.discards(),
            [Discard::ConflictingAddress {
                previous: from(1),
                received: from(2)
            }]
        );

        assert!(!h.segment(broadcast(), true, false, 40, &[1]));
        assert_eq!(
            h.discards(),
            [Discard::BroadcastSeries {
                fir: true,
                fin: false
            }]
        );
        h.assert_recovers(41);
        assert!(h.discards().is_empty());
    }

    #[test]
    fn repeat_of_the_first_segment_without_fir_abandons_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(0, &[1]));
        assert_eq!(h.discards(), [Discard::DifferentRepeat { seq: 0 }]);
        assert!(!h.last(1, &[2]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn discards_display_their_log_messages() {
        let source = EndpointAddress::raw(1);
        let cases = [
            (
                Discard::NewSeries { len: 3, source },
                "received FIR - dropping 3 assembled bytes from 1",
            ),
            (
                Discard::BroadcastSeries {
                    fir: false,
                    fin: true,
                },
                "ignoring broadcast frame with transport header fir: false and fin: true",
            ),
            (
                Discard::NoSeries { source },
                "ignoring non-FIR segment from 1 with no previous FIR",
            ),
            (
                Discard::Repeat { seq: 1 },
                "ignoring repeated segment with seq: 1",
            ),
            (
                Discard::DifferentRepeat { seq: 1 },
                "segment with repeated seq: 1 differs from the previous segment",
            ),
            (
                Discard::UnexpectedSeq {
                    expected: 11,
                    received: 12,
                },
                "expected seq: 11 but received seq: 12",
            ),
            (
                Discard::Overflow { len: 9 },
                "buffer overflow with 9 bytes to write",
            ),
            (
                Discard::Pending,
                "ignoring segment while a fragment waits to be read",
            ),
        ];
        for (discard, message) in cases {
            assert_eq!(discard.to_string(), message);
        }
        let conflicting = Discard::ConflictingAddress {
            previous: from(1),
            received: from(2),
        };
        assert!(conflicting.to_string().starts_with("conflicting addresses"));
    }
}
