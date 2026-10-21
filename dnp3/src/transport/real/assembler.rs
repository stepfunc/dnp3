use crate::link::header::FrameInfo;
use crate::transport::real::header::Header;
use crate::transport::{Fragment, FragmentAddr, FragmentInfo};

/// Reassembles transport segments into fragments, following the transport reception state table
/// of IEEE 1815-2012 8.2.4 as replaced by TB2013-003.
///
/// Extensions to the table:
/// - a unicast segment that isn't FIR, from a different source or physical address, abandons the
///   series
/// - broadcast segments are accepted only as a single FIR/FIN segment, and others are ignored
pub(crate) struct Assembler {
    state: State,
    frame_id: u32,
    buffer: Box<[u8]>,
    /// length of the series or fragment in `buffer`; meaningful while `Running` or `Complete`.
    /// Only `extend` grows it, to an end that `buffer` was just checked to contain.
    len: usize,
}

#[derive(Copy, Clone)]
enum State {
    Idle,
    /// a series is in progress, and `last` and `last_len` describe its most recent segment
    Running {
        info: FrameInfo,
        last: Header,
        last_len: usize,
    },
    /// the buffer holds a fragment waiting to be taken
    Complete(FragmentInfo),
}

pub(crate) enum AssemblyState {
    Complete,
    ReadMore,
}

struct Overflow;

impl Assembler {
    pub(crate) fn new(max_fragment_size: usize) -> Self {
        Self {
            state: State::Idle,
            frame_id: 0,
            buffer: vec![0; max_fragment_size].into_boxed_slice(),
            len: 0,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.abandon();
    }

    pub(crate) fn peek(&self) -> Option<Fragment<'_>> {
        match self.state {
            State::Complete(info) => Some(Fragment {
                info,
                data: self.fragment(),
            }),
            _ => None,
        }
    }

    pub(crate) fn pop(&mut self) -> Option<Fragment<'_>> {
        match self.state {
            State::Complete(info) => {
                // `len` is kept so the returned fragment still covers the data; the next FIR resets it
                self.state = State::Idle;
                Some(Fragment {
                    info,
                    data: self.fragment(),
                })
            }
            _ => None,
        }
    }

    pub(crate) fn assemble(
        &mut self,
        info: FrameInfo,
        header: Header,
        data: &[u8],
    ) -> AssemblyState {
        // FIR always clears the previous assembly
        if header.fir {
            if let State::Running { info, .. } = self.state {
                tracing::warn!(
                    "transport: received FIR - dropping {} assembled bytes from {}",
                    self.len,
                    info.source
                );
            }
            self.abandon();
        }

        if info.broadcast.is_some() {
            if header.fir && header.fin {
                self.append(info, header, data);
            } else {
                tracing::warn!(
                    "ignoring broadcast frame with transport header fir: {} and fin: {}",
                    header.fir,
                    header.fin
                );
            }
            return self.assembly_state();
        }

        match self.state {
            State::Idle | State::Complete(_) => {
                if header.fir {
                    self.append(info, header, data);
                } else {
                    tracing::warn!(
                        "transport: ignoring non-FIR segment from {} with no previous FIR",
                        info.source
                    );
                }
            }
            State::Running {
                info: previous,
                last,
                last_len,
            } => {
                if info != previous {
                    tracing::warn!("transport: conflicting addresses, previous segment with {:?}, but received {:?}", previous, info);
                    self.abandon();
                } else if header.seq.value() == last.seq.value() {
                    let repeat = header.to_u8() == last.to_u8()
                        && data.len() == last_len
                        && self.fragment().ends_with(data);
                    if repeat {
                        tracing::warn!(
                            "transport: ignoring repeated segment with seq: {}",
                            header.seq.value()
                        );
                    } else {
                        tracing::warn!("transport: segment with repeated seq: {} differs from the previous segment", header.seq.value());
                        self.abandon();
                    }
                } else if header.seq.value() != last.seq.next() {
                    tracing::warn!(
                        "transport: expected seq: {} but received seq: {}",
                        last.seq.next(),
                        header.seq.value()
                    );
                    self.abandon();
                } else {
                    self.append(info, header, data);
                }
            }
        }

        self.assembly_state()
    }

    fn assembly_state(&self) -> AssemblyState {
        match self.state {
            State::Complete(_) => AssemblyState::Complete,
            _ => AssemblyState::ReadMore,
        }
    }

    fn fragment(&self) -> &[u8] {
        self.buffer.get(..self.len).unwrap_or_default()
    }

    fn abandon(&mut self) {
        self.state = State::Idle;
        self.len = 0;
    }

    fn append(&mut self, info: FrameInfo, header: Header, data: &[u8]) {
        if self.extend(data).is_err() {
            tracing::warn!(
                "transport buffer overflow with {} bytes to write",
                data.len()
            );
            self.abandon();
            return;
        }

        if !header.fin {
            self.state = State::Running {
                info,
                last: header,
                last_len: data.len(),
            };
        } else if self.len == 0 {
            // an empty fragment isn't passed to the application layer
            self.state = State::Idle;
        } else {
            let addr = FragmentAddr {
                link: info.source,
                phys: info.phys_addr,
            };
            let id = self.frame_id;
            self.frame_id = self.frame_id.wrapping_add(1);
            self.state = State::Complete(FragmentInfo::new(id, addr, info.broadcast));
        }
    }

    fn extend(&mut self, data: &[u8]) -> Result<(), Overflow> {
        let end = self.len.checked_add(data.len()).ok_or(Overflow)?;
        let dest = self.buffer.get_mut(self.len..end).ok_or(Overflow)?;
        for (dest, src) in dest.iter_mut().zip(data) {
            *dest = *src;
        }
        self.len = end;
        Ok(())
    }
}

/// Tests named after the rows of the transport reception state table as replaced by TB2013-003
#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::header::{BroadcastConfirmMode, FrameType};
    use crate::link::EndpointAddress;
    use crate::transport::real::sequence::Sequence;
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
    }

    impl Harness {
        fn new() -> Self {
            Self {
                assembler: Assembler::new(MAX),
            }
        }

        /// returns true if the segment completed a fragment
        fn segment(&mut self, info: FrameInfo, fir: bool, fin: bool, seq: u8, data: &[u8]) -> bool {
            let header = Header::new(fin, fir, Sequence::new(seq));
            matches!(
                self.assembler.assemble(info, header, data),
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

        fn pop(&mut self) -> Option<(u32, Vec<u8>)> {
            self.assembler
                .pop()
                .map(|fragment| (fragment.info.id, fragment.data.to_vec()))
        }

        fn pop_data(&mut self) -> Option<Vec<u8>> {
            self.pop().map(|(_, data)| data)
        }

        /// a fresh single-segment fragment still assembles
        fn assert_recovers(&mut self, seq: u8) {
            assert!(self.single(seq, &[0xEE]));
            assert_eq!(self.pop_data(), Some(vec![0xEE]));
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
        assert_eq!(h.pop_data(), Some(vec![1, 2, 3]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn row_3_single_segment_of_exactly_the_maximum_size_is_passed_up() {
        let mut h = Harness::new();
        assert!(h.single(0, &[7; MAX]));
        assert_eq!(h.pop_data(), Some(vec![7; MAX]));
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
        assert_eq!(h.pop_data(), Some(vec![1, 2]));
    }

    #[test]
    fn row_6_first_segment_starts_a_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert_eq!(h.pop(), None);
        assert!(h.last(1, &[2]));
        assert_eq!(h.pop_data(), Some(vec![1, 2]));
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
        assert!(h.last(2, &[4]));
        assert_eq!(h.pop_data(), Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn row_8_identical_repeat_of_an_empty_segment_is_discarded() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[]));
        assert!(!h.middle(1, &[]));
        assert!(h.last(2, &[2]));
        assert_eq!(h.pop_data(), Some(vec![1, 2]));
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
        assert_eq!(h.pop_data(), Some(vec![1, 2]));
    }

    #[test]
    fn row_11_segments_of_varying_sizes_append_in_order() {
        let mut h = Harness::new();
        assert!(!h.first(10, &[1, 2, 3]));
        assert!(!h.middle(11, &[4]));
        assert!(!h.middle(12, &[5, 6]));
        assert!(h.last(13, &[7]));
        assert_eq!(h.pop_data(), Some(vec![1, 2, 3, 4, 5, 6, 7]));
    }

    #[test]
    fn row_11_middle_segment_reaching_exactly_the_maximum_size_appends() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1; MAX - 2]));
        assert!(!h.middle(1, &[2; 2]));
        assert!(h.last(2, &[]));
        assert_eq!(h.pop_data().map(|x| x.len()), Some(MAX));
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
        assert_eq!(h.pop_data(), Some(vec![1, 2]));
    }

    #[test]
    fn row_15_final_segment_reaching_exactly_the_maximum_size_passes_up_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1; MAX - 1]));
        assert!(h.last(1, &[2]));
        assert_eq!(h.pop_data().map(|x| x.len()), Some(MAX));
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
        assert_eq!(h.pop_data(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn row_18_empty_first_segment_restarts_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2]));
        assert!(!h.first(5, &[]));
        assert!(h.last(6, &[3]));
        assert_eq!(h.pop_data(), Some(vec![3]));
    }

    #[test]
    fn row_19_first_segment_restarts_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2]));
        assert!(!h.first(9, &[3]));
        assert!(h.last(10, &[4]));
        assert_eq!(h.pop_data(), Some(vec![3, 4]));
    }

    #[test]
    fn row_19_repeated_first_segment_restarts_the_series() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(!h.middle(1, &[2]));
        assert!(!h.first(0, &[1]));
        assert!(h.last(1, &[3]));
        assert_eq!(h.pop_data(), Some(vec![1, 3]));
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
        assert_eq!(h.pop_data(), Some(vec![2]));
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
        assert_eq!(h.pop_data(), Some(vec![1, 2]));
        assert!(!h.last(1, &[2]));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn repeated_single_segment_is_passed_up_again() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1]));
        assert_eq!(h.pop_data(), Some(vec![1]));
        assert!(h.single(0, &[1]));
        assert_eq!(h.pop_data(), Some(vec![1]));
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
        assert_eq!(h.pop_data(), Some(vec![1, 2]));
    }

    #[test]
    fn broadcast_single_segment_replaces_a_series_in_progress() {
        let mut h = Harness::new();
        assert!(!h.first(0, &[1]));
        assert!(h.segment(broadcast(), true, true, 5, &[9]));
        assert_eq!(h.pop_data(), Some(vec![9]));
        assert!(!h.last(1, &[2]));
    }

    #[test]
    fn peek_leaves_the_fragment_and_pop_takes_it_once() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1]));
        assert_eq!(h.assembler.peek().map(|f| f.data.to_vec()), Some(vec![1]));
        assert_eq!(h.assembler.peek().map(|f| f.data.to_vec()), Some(vec![1]));
        assert_eq!(h.pop_data(), Some(vec![1]));
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
    fn reset_discards_a_completed_fragment() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1]));
        h.assembler.reset();
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn fragment_ids_count_only_completed_fragments() {
        let mut h = Harness::new();
        assert!(h.single(0, &[1]));
        assert_eq!(h.pop().map(|(id, _)| id), Some(0));
        // discarded and empty segments don't use an id
        assert!(!h.middle(1, &[2]));
        assert!(!h.single(2, &[7; MAX + 1]));
        assert!(!h.single(3, &[]));
        assert!(h.single(4, &[1]));
        assert_eq!(h.pop().map(|(id, _)| id), Some(1));
    }

    #[test]
    fn fragment_ids_wrap() {
        let mut h = Harness::new();
        h.assembler.frame_id = u32::MAX;
        assert!(h.single(0, &[1]));
        assert_eq!(h.pop().map(|(id, _)| id), Some(u32::MAX));
        assert!(h.single(1, &[1]));
        assert_eq!(h.pop().map(|(id, _)| id), Some(0));
    }
}
