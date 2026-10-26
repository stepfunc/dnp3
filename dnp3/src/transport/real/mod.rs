// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::transport::Transport;

pub(crate) mod assembler;
pub(crate) mod display;
pub(crate) mod header;
pub(crate) mod reader;
pub(crate) mod sequence;
#[cfg(test)]
mod test_support;
pub(crate) mod writer;

pub(crate) mod constants {
    pub(crate) const FIN_MASK: u8 = 0b1000_0000;
    pub(crate) const FIR_MASK: u8 = 0b0100_0000;
}

/// DNP3 link-layer framing with transport segmentation and reassembly
pub(crate) struct RealTransport;

impl Transport for RealTransport {
    type Reader = reader::Reader;
    type Writer = writer::Writer;

    fn new(
        link_modes: LinkModes,
        role: LinkRole,
        address: EndpointAddress,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer) {
        (
            reader::Reader::new(link_modes, role, address, rx_buffer_size),
            writer::Writer::new(role, address),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::test_support::*;
    use super::*;
    use crate::decode::DecodeLevel;
    use crate::link::header::AnyAddress;
    use crate::link::test_data;
    use crate::phys::{PhysAddr, PhysLayer};
    use crate::transport::{FragmentAddr, TransportData, TransportRead, TransportWrite};

    /// a data frame carrying a whole fragment in one segment
    fn fragment_frame(
        from_master: bool,
        destination: AnyAddress,
        source: EndpointAddress,
        seq: u8,
        data: &[u8],
    ) -> Vec<u8> {
        let header = link_header(from_master, UNCONFIRMED_USER_DATA, destination, source);
        test_data::data_frame(header, test_data::segment(true, true, seq), data)
    }

    #[tokio::test(start_paused = true)]
    async fn master_writes_frames_from_the_master() {
        let mut h = Harness::new(master());
        h.write(OUTSTATION, &[1, 2, 3]).await;
        h.write_link_status_request(OUTSTATION).await;
        assert_eq!(
            h.writes(),
            [
                fragment_frame(true, OUTSTATION.wrap(), MASTER, 0, &[1, 2, 3]),
                header_only(true, REQUEST_LINK_STATUS, OUTSTATION.wrap(), MASTER),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn outstation_writes_frames_from_the_outstation() {
        for self_address in [false, true] {
            let mut h = Harness::new(outstation(self_address));
            h.write(MASTER, &[1, 2, 3]).await;
            h.write_link_status_request(MASTER).await;
            assert_eq!(
                h.writes(),
                [
                    fragment_frame(false, MASTER.wrap(), OUTSTATION, 0, &[1, 2, 3]),
                    header_only(false, REQUEST_LINK_STATUS, MASTER.wrap(), OUTSTATION),
                ]
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn master_answers_link_status_requests() {
        let mut h = Harness::new(master());
        let request = header_only(false, REQUEST_LINK_STATUS, MASTER.wrap(), OUTSTATION);
        assert_eq!(
            h.deliver(&[request]).await,
            Some(Item::LinkStatusRequest(OUTSTATION))
        );
        assert_eq!(
            h.writes(),
            [header_only(true, LINK_STATUS, OUTSTATION.wrap(), MASTER)]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn master_reads_link_status_responses() {
        let mut h = Harness::new(master());
        let response = header_only(false, LINK_STATUS, MASTER.wrap(), OUTSTATION);
        assert_eq!(
            h.deliver(&[response]).await,
            Some(Item::LinkStatusResponse(OUTSTATION))
        );
        assert!(h.writes().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn outstation_without_self_address_ignores_frames_sent_to_self_address() {
        let mut h = Harness::new(outstation(false));
        let frames = [
            fragment_frame(true, AnyAddress::SelfAddress, MASTER, 0, &[1]),
            header_only(true, REQUEST_LINK_STATUS, AnyAddress::SelfAddress, MASTER),
            fragment_frame(true, OUTSTATION.wrap(), MASTER, 1, &[2]),
        ];
        assert_eq!(h.deliver(&frames).await, Some(Item::Fragment(vec![2])));
        assert!(h.writes().is_empty());
    }

    /// Fragments written by the transport writer, as the transport reader delivers them after
    /// segments are repeated, dropped or interleaved with another source's series
    ///
    /// The writer writes a fragment, then a one-byte sentinel; one edit changes the frames
    /// between them, and the reader gets the result through the link layer. Two oracles, both
    /// independent of the code under test: [`expected`] predicts what the reader delivers from
    /// the transport rules alone, and [`headers`] gives the transport header of every segment
    /// the writer wrote for the fragment and the sentinel from literal FIR (0x40), FIN (0x80) and
    /// sequence values. The writer and the assembler share the header encoding and the sequence
    /// arithmetic, so the delivery model alone can't see a mistake made the same way on both
    /// sides; the literal headers can.
    ///
    /// The generator covers both directions, three distinct addresses, a first sequence number
    /// often near the wrap (so it wraps within the fragment), fragments of 1 to 2048 bytes plus
    /// the segment and buffer boundaries (one segment, two, nine, one past the reader's buffer),
    /// and each edit at the first, middle, last or any segment. Known limits: unconfirmed user
    /// data only, one remote, no physical addresses or broadcasts (the assembler's property
    /// covers those), and the headers of the changed repeat and of the other source's fragment,
    /// written by separate writers, aren't checked.
    mod properties {
        use proptest::prelude::*;
        use proptest::sample::Index;

        use super::*;
        use crate::link::constant::MAX_APP_BYTES_PER_FRAME;
        use crate::link::header::BroadcastConfirmMode;
        use crate::util::test_support::{insert_at, remove_at};

        const RX_BUFFER_SIZE: usize = 2048;
        /// the transport header is the first byte after the 10-byte link header
        const TRANSPORT_HEADER: usize = 10;
        /// written after the edited fragment, so the reader always has something to deliver
        const SENTINEL: [u8; 1] = [0xAA];

        /// a position among the segments, with the ends and the middle weighted
        #[derive(Clone, Debug)]
        enum Position {
            First,
            Middle,
            Last,
            Any(Index),
        }

        impl Position {
            /// a position in `0..count`, where `count` isn't zero
            fn resolve(&self, count: usize) -> usize {
                match self {
                    Self::First => 0,
                    Self::Middle => count / 2,
                    Self::Last => count.saturating_sub(1),
                    Self::Any(x) => x.index(count),
                }
            }
        }

        #[derive(Clone, Debug)]
        enum Edit {
            None,
            /// send a segment twice in a row
            Repeat(Position),
            /// send a segment again with the same header and length but a changed first byte
            ChangedRepeat(Position),
            /// leave a segment out
            Drop(Position),
            /// before a segment, or after the last when `at` is `None`, send the first `count`
            /// segments of another source's fragment, or all of them when `count` is `None`
            Foreign {
                at: Option<Position>,
                fragment: Vec<u8>,
                count: Option<Index>,
            },
        }

        #[derive(Clone, Debug)]
        struct Case {
            /// true if the master writes and the outstation reads
            master_writes: bool,
            local: EndpointAddress,
            remote: EndpointAddress,
            foreign: EndpointAddress,
            /// the transport sequence number of the fragment's first segment
            first_seq: u8,
            fragment: Vec<u8>,
            edit: Edit,
        }

        /// what the reader delivered
        #[derive(Debug, PartialEq)]
        enum Item {
            Fragment {
                source: EndpointAddress,
                phys: PhysAddr,
                broadcast: Option<BroadcastConfirmMode>,
                data: Vec<u8>,
            },
            LinkLayerMessage,
        }

        impl Item {
            fn fragment(source: EndpointAddress, data: &[u8]) -> Self {
                Self::Fragment {
                    source,
                    phys: PhysAddr::None,
                    broadcast: None,
                    data: data.to_vec(),
                }
            }
        }

        /// the number of segments that carry `len` bytes
        fn segments(len: usize) -> usize {
            len.div_ceil(MAX_APP_BYTES_PER_FRAME)
        }

        /// The transport headers of the fragment's segments and of the sentinel, from the wire
        /// encoding alone: FIR (0x40) on the first segment of each, FIN (0x80) on the last, and a
        /// sequence number that counts up from `first_seq` modulo 64
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "a few segments past a sequence number below 64"
        )]
        fn headers(case: &Case) -> Vec<u8> {
            let count = segments(case.fragment.len());
            let seq = |i: usize| u8::try_from((usize::from(case.first_seq) + i) % 64).unwrap();
            let mut headers: Vec<u8> = (0..count)
                .map(|i| {
                    let fir = if i == 0 { 0x40 } else { 0 };
                    let fin = if i + 1 == count { 0x80 } else { 0 };
                    fir | fin | seq(i)
                })
                .collect();
            headers.push(0xC0 | seq(count));
            headers
        }

        /// Where the edit goes, once the number of segments is known
        enum Resolved<'a> {
            None,
            Repeat(usize),
            ChangedRepeat(usize),
            Drop(usize),
            /// the first `count` of the `total` segments of `fragment`, before segment `at`
            Foreign {
                at: usize,
                fragment: &'a [u8],
                count: usize,
                total: usize,
            },
        }

        fn resolve(edit: &Edit, segments: usize) -> Resolved<'_> {
            match edit {
                Edit::None => Resolved::None,
                Edit::Repeat(x) => Resolved::Repeat(x.resolve(segments)),
                Edit::ChangedRepeat(x) => Resolved::ChangedRepeat(x.resolve(segments)),
                Edit::Drop(x) => Resolved::Drop(x.resolve(segments)),
                Edit::Foreign {
                    at,
                    fragment,
                    count,
                } => {
                    let total = self::segments(fragment.len());
                    Resolved::Foreign {
                        at: at.as_ref().map_or(segments, |x| x.resolve(segments)),
                        fragment,
                        count: count
                            .as_ref()
                            .map_or(total, |x| x.index(total).saturating_add(1)),
                        total,
                    }
                }
            }
        }

        /// the fragment with the first byte of segment `i` changed
        fn changed(fragment: &[u8], i: usize) -> Vec<u8> {
            let mut changed = fragment.to_vec();
            let first = i.checked_mul(MAX_APP_BYTES_PER_FRAME);
            if let Some(x) = first.and_then(|first| changed.get_mut(first)) {
                *x = !*x;
            }
            changed
        }

        /// The model: what the reader delivers, from the transport rules alone. A FIR segment
        /// starts a new series from any source, a segment that doesn't continue the series in
        /// progress abandons it, and the sentinel always arrives.
        fn expected(case: &Case) -> Vec<Item> {
            let fits = case.fragment.len() <= RX_BUFFER_SIZE;
            let original = || {
                fits.then(|| Item::fragment(case.local, &case.fragment))
                    .into_iter()
            };
            let segments = segments(case.fragment.len());
            let mut items = Vec::new();
            match resolve(&case.edit, segments) {
                Resolved::None => items.extend(original()),
                // a repeated FIR restarts the series with the same bytes, a repeated segment
                // that isn't FIR is ignored, and a repeated single segment is another fragment
                Resolved::Repeat(_) => {
                    items.extend(original());
                    if segments == 1 {
                        items.extend(original());
                    }
                }
                // a changed repeat abandons the series in progress (TB2013-003 row 9), but a
                // changed FIR starts a new one and a changed single segment is another fragment
                Resolved::ChangedRepeat(i) => {
                    let changed = || {
                        fits.then(|| Item::fragment(case.local, &changed(&case.fragment, i)))
                            .into_iter()
                    };
                    if segments == 1 {
                        items.extend(original());
                        items.extend(changed());
                    } else if i == 0 {
                        items.extend(changed());
                    } else if i == segments.saturating_sub(1) {
                        // the fragment is complete before the repeat arrives
                        items.extend(original());
                    }
                }
                Resolved::Drop(_) => {}
                // the foreign series abandons the original if it starts inside it, and is
                // delivered only if complete
                Resolved::Foreign {
                    at,
                    fragment,
                    count,
                    total,
                } => {
                    let foreign = (count == total).then(|| Item::fragment(case.foreign, fragment));
                    if at == 0 {
                        items.extend(foreign);
                        items.extend(original());
                    } else if at == segments {
                        items.extend(original());
                        items.extend(foreign);
                    } else {
                        items.extend(foreign);
                    }
                }
            }
            items.push(Item::fragment(case.local, &SENTINEL));
            items
        }

        fn writer_role(master_writes: bool) -> LinkRole {
            if master_writes {
                LinkRole::Master
            } else {
                LinkRole::Outstation {
                    self_address: false,
                }
            }
        }

        /// the frames written for each fragment, in order
        async fn write(
            role: LinkRole,
            local: EndpointAddress,
            remote: EndpointAddress,
            fragments: &[&[u8]],
        ) -> Vec<Vec<Vec<u8>>> {
            let (io, mut handle) = sfio_tokio_mock_io::mock();
            let mut io = PhysLayer::mock(io);
            let mut writer = writer::Writer::new(role, local);
            let destination = FragmentAddr {
                link: remote,
                phys: PhysAddr::None,
            };
            let mut frames = Vec::new();
            for fragment in fragments {
                writer
                    .write(&mut io, DecodeLevel::nothing(), destination, fragment)
                    .await
                    .unwrap();
                frames.push(drain_writes(&mut handle));
            }
            frames
        }

        /// the edited frames the reader receives, and the transport headers the writer wrote for
        /// the fragment's segments and the sentinel
        async fn stream(case: &Case) -> (Vec<Vec<u8>>, Vec<u8>) {
            let role = writer_role(case.master_writes);
            // single-segment fragments advance the writer's sequence number to `first_seq`
            let mut fragments: Vec<&[u8]> = vec![&[0]; usize::from(case.first_seq)];
            fragments.push(&case.fragment);
            fragments.push(&SENTINEL);
            let mut written = write(role, case.local, case.remote, &fragments).await;
            let sentinel = written.pop().unwrap();
            let mut frames = written.pop().unwrap();
            let segments = frames.len();
            let headers = frames
                .iter()
                .chain(&sentinel)
                .map(|x| x[TRANSPORT_HEADER])
                .collect();

            match resolve(&case.edit, segments) {
                Resolved::None => {}
                Resolved::Repeat(i) => {
                    let repeated = frames[i].clone();
                    insert_at(&mut frames, i, [repeated]);
                }
                Resolved::ChangedRepeat(i) => {
                    let changed = changed(&case.fragment, i);
                    let mut fragments: Vec<&[u8]> = vec![&[0]; usize::from(case.first_seq)];
                    fragments.push(&changed);
                    let mut written = write(role, case.local, case.remote, &fragments).await;
                    let changed = written.pop().unwrap();
                    insert_at(&mut frames, i.saturating_add(1), [changed[i].clone()]);
                }
                Resolved::Drop(i) => {
                    remove_at(&mut frames, i);
                }
                Resolved::Foreign {
                    at,
                    fragment,
                    count,
                    ..
                } => {
                    let mut foreign = write(role, case.foreign, case.remote, &[fragment]).await;
                    let mut foreign = foreign.pop().unwrap();
                    foreign.truncate(count);
                    insert_at(&mut frames, at, foreign);
                }
            }
            frames.extend(sentinel);
            (frames, headers)
        }

        /// everything the reader delivers from the frames, and what it wrote back
        async fn read(case: &Case, frames: &[Vec<u8>]) -> (Vec<Item>, Vec<Vec<u8>>) {
            let (io, mut handle) = sfio_tokio_mock_io::mock();
            let mut io = PhysLayer::mock(io);
            let mut reader = reader::Reader::new(
                LinkModes::test(),
                writer_role(!case.master_writes),
                case.remote,
                RX_BUFFER_SIZE,
            );
            for frame in frames {
                handle.read(frame);
            }

            let mut items = Vec::new();
            // each completed read consumes at least one frame
            for _ in 0..=frames.len() {
                let read = tokio::time::timeout(
                    Duration::from_secs(1),
                    reader.read(&mut io, DecodeLevel::nothing()),
                )
                .await;
                let Ok(result) = read else {
                    break;
                };
                result.unwrap();
                items.extend(reader.pop().map(|data| match data {
                    TransportData::Fragment(x) => Item::Fragment {
                        source: x.info.addr.link,
                        phys: x.info.addr.phys,
                        broadcast: x.info.broadcast,
                        data: x.data.to_vec(),
                    },
                    TransportData::LinkLayerMessage(_) => Item::LinkLayerMessage,
                }));
            }

            (items, drain_writes(&mut handle))
        }

        /// the transport headers written (see [`stream`]), what the reader delivered, and what it
        /// wrote back
        fn run(case: &Case) -> (Vec<u8>, Vec<Item>, Vec<Vec<u8>>) {
            crate::util::test_support::run_paused(async {
                let (frames, headers) = stream(case).await;
                let (items, writes) = read(case, &frames).await;
                (headers, items, writes)
            })
        }

        #[expect(
            clippy::arithmetic_side_effects,
            reason = "boundaries of a few segments"
        )]
        fn fragment_len() -> impl Strategy<Value = usize> {
            let per_frame = MAX_APP_BYTES_PER_FRAME;
            prop_oneof![
                3 => 1..=RX_BUFFER_SIZE,
                2 => prop::sample::select(vec![
                    1,
                    per_frame - 1,
                    per_frame,
                    per_frame + 1,
                    2 * per_frame,
                    2 * per_frame + 1,
                    RX_BUFFER_SIZE - 1,
                    RX_BUFFER_SIZE,
                    RX_BUFFER_SIZE + 1,
                    9 * per_frame,
                ]),
            ]
        }

        /// `len` bytes counting up from a seed, which shrink along with the length
        fn bytes(len: impl Strategy<Value = usize>) -> impl Strategy<Value = Vec<u8>> {
            (len, any::<u8>()).prop_map(|(len, seed)| {
                (0..len)
                    .map(|x| seed.wrapping_add(x.to_le_bytes()[0]))
                    .collect()
            })
        }

        fn position() -> impl Strategy<Value = Position> {
            prop_oneof![
                1 => Just(Position::First),
                1 => Just(Position::Middle),
                1 => Just(Position::Last),
                3 => any::<Index>().prop_map(Position::Any),
            ]
        }

        fn edit() -> impl Strategy<Value = Edit> {
            let foreign_fragment = bytes(prop_oneof![1..=3 * MAX_APP_BYTES_PER_FRAME, 1usize..=4]);
            prop_oneof![
                1 => Just(Edit::None),
                2 => position().prop_map(Edit::Repeat),
                2 => position().prop_map(Edit::ChangedRepeat),
                2 => position().prop_map(Edit::Drop),
                3 => (
                    prop::option::of(position()),
                    foreign_fragment,
                    prop::option::of(any::<Index>()),
                )
                    .prop_map(|(at, fragment, count)| Edit::Foreign {
                        at,
                        fragment,
                        count,
                    }),
            ]
        }

        #[expect(
            clippy::arithmetic_side_effects,
            reason = "offsets below 0xFFF0 between addresses"
        )]
        fn case() -> impl Strategy<Value = Case> {
            (
                any::<bool>(),
                // three distinct addresses below 0xFFF0, in a reproducible order
                0u16..0xFFF0,
                1u16..0x7FF8,
                1u16..0x7FF8,
                // the sequence number often wraps within the fragment
                prop_oneof![0u8..64, 54u8..64],
                bytes(fragment_len()),
                edit(),
            )
                .prop_map(
                    |(master_writes, local, to_remote, to_foreign, first_seq, fragment, edit)| {
                        let address = |offset: u16| {
                            let x = (u32::from(local) + u32::from(offset)) % 0xFFF0;
                            EndpointAddress::raw(u16::try_from(x).unwrap())
                        };
                        Case {
                            master_writes,
                            local: EndpointAddress::raw(local),
                            remote: address(to_remote),
                            foreign: address(to_remote + to_foreign),
                            first_seq,
                            fragment,
                            edit,
                        }
                    },
                )
        }

        /// the transport headers of [`example`]: FIR with sequence 62, 63, FIN with 0, then the
        /// sentinel with FIR, FIN and 1
        const EXAMPLE_HEADERS: [u8; 4] = [0x7E, 0x3F, 0x80, 0xC1];

        /// a three-segment fragment from the master whose sequence numbers wrap
        fn example(edit: Edit) -> Case {
            Case {
                master_writes: true,
                local: MASTER,
                remote: OUTSTATION,
                foreign: EndpointAddress::raw(2),
                first_seq: 62,
                fragment: (0..3 * MAX_APP_BYTES_PER_FRAME)
                    .map(|x| x.to_le_bytes()[0])
                    .collect(),
                edit,
            }
        }

        #[test]
        fn model_expects_only_the_series_that_starts_last() {
            let case = example(Edit::Foreign {
                at: Some(Position::Last),
                fragment: vec![7],
                count: None,
            });
            let expected = [
                Item::fragment(EndpointAddress::raw(2), &[7]),
                Item::fragment(MASTER, &SENTINEL),
            ];
            assert_eq!(super::properties::expected(&case), expected);
            assert_eq!(
                run(&case),
                (EXAMPLE_HEADERS.into(), expected.into(), Vec::new())
            );
        }

        #[test]
        fn model_rejects_a_fragment_missing_a_segment() {
            let case = example(Edit::Drop(Position::Middle));
            let expected = [Item::fragment(MASTER, &SENTINEL)];
            assert_eq!(super::properties::expected(&case), expected);
            assert_eq!(
                run(&case),
                (EXAMPLE_HEADERS.into(), expected.into(), Vec::new())
            );
        }

        #[test]
        fn model_rejects_a_fragment_with_a_changed_repeat() {
            let case = example(Edit::ChangedRepeat(Position::Middle));
            let expected = [Item::fragment(MASTER, &SENTINEL)];
            assert_eq!(super::properties::expected(&case), expected);
            assert_eq!(
                run(&case),
                (EXAMPLE_HEADERS.into(), expected.into(), Vec::new())
            );
        }

        proptest! {
            #![proptest_config(crate::util::test_support::proptest_config(128))]

            #[test]
            fn reader_delivers_what_the_writer_wrote(case in case()) {
                let (headers, delivered, writes) = run(&case);
                prop_assert_eq!(headers, self::headers(&case));
                prop_assert_eq!(delivered, expected(&case));
                // unconfirmed user data gets no reply
                prop_assert!(writes.is_empty());
            }
        }
    }
}
