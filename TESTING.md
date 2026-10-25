# Testing

This file is for maintainers of the `dnp3` crate. It says how the crate is tested, indexes every
property test, records which layer owns which assertions, and lists the known gaps. Each property's
module doc is the detailed source: its claim, its oracle, what its generator can and can't produce,
the cases it is built to reach, and its known gaps. The commands to build, test and lint are in
[`CLAUDE.md`](CLAUDE.md).

## How the crate is tested

- **Examples.** Unit tests next to the code, and tests driven through a harness under
  `dnp3/src/master/tests` and `dnp3/src/outstation/tests`. They pin wire bytes, IIN bits and
  callback order for particular cases, and compare structured values rather than formatted text or
  logs. Tests that drive a session run under `util::test_support::watchdog`.
- **Fixed op lists through models.** Some examples run a hand-picked list of operations through the
  same model a property uses (the link layer, the transport assembler, the event buffer and the
  outstation session). They make a case the property reaches only in some runs deterministic, and
  pin its bytes.
- **Properties.** Generated cases checked against an oracle with `proptest`, indexed below.
- **No-panic check.** `dnp3/src/no_panic_check.rs` links the application parsers, master
  extraction, the transport assembler, the link frame reader and the link layer's header processing
  so that the build fails unless the optimizer removes every path to a panic.
- **Binding smoke tests.** The C, C++, .NET and Java bindings each run an outstation and a master in
  one process over TCP loopback and check a class 0 read and a direct operate, against libraries
  built like the shipped ones.
- **Conformance.** The DNP3 IED certification procedures run against the outstation through the
  Java bindings, on release tags, weekly, on demand and on PRs labeled `conformance`. See
  `CLAUDE.md` for when to label a PR.

## Properties

Every `proptest!` block sets its case budget with `util::test_support::proptest_config(cases)`.
The budget is the number of cases one run of the property generates; `PROPTEST_CASES` overrides it
for every property at once.

Oracles:

- **Independent**: a model or encoding written from the rules, sharing no code with what it checks.
- **Round trip**: production writes and production parses back. It can't see a mistake made the
  same way on both sides, so fixed bytes in examples pin the format.
- **Differential**: two production paths compared with each other.
- **Relation**: laws the code must satisfy, such as symmetry, without an expected value.

Paths are relative to `dnp3/src`.

### Application layer

| Property | File | Claim | Oracle | Cases |
|---|---|---|---|---|
| `packed_sequences_yield_the_bits_of_each_byte_in_order` | `app/parse/bit.rs` | Packed bits and double bits: bytes read, values in order with their indices, one byte short fails | Independent (literal masks) | 256 |
| `count_sequence_yields_each_object_once` | `app/parse/count.rs` | Count sequences: bytes read, each object once, a single value only for a count of one | Independent (test encodes the objects) | 256 |
| `ranged_sequence_yields_each_index_once` | `app/parse/range.rs` | Ranged sequences: bytes read, each value with its index, one byte short fails | Independent (test encodes the values) | 256 |
| `attributes_round_trip_in_the_smallest_encoding` | `app/attr.rs` | Reserved variations and long values are rejected unwritten; others use the smallest encoding and parse back | Independent for type code, length and errors; round trip for values | 512 |
| `attributes_need_exactly_their_encoded_size` | `app/attr.rs` | An attribute production accepts as encodable fits exactly its encoded size and fails one byte short | Independent size model, applied only where production's `check_encodable` succeeds (the round-trip property checks acceptance) | 512 |
| `analog_conversions_match_the_model` | `app/measurement.rs` | i16, i32 and f32 conversions of the analog types, and OVER_RANGE | Independent (model without `as`) | 1024 |
| `g70v2_round_trips_and_rejects_bad_encodings` | `app/file/codec_tests.rs` | g70v2 parses back; wrong offsets or lengths and bad UTF-8 are bad encodings | Round trip, independent corruptions | 256 |
| `g70v3_round_trips_and_rejects_bad_encodings` | `app/file/codec_tests.rs` | g70v3 the same, with its file name offset | Round trip, independent corruptions | 256 |
| `g70v4_round_trips_and_rejects_bad_encodings` | `app/file/codec_tests.rs` | g70v4 parses back; bad UTF-8 is a bad encoding | Round trip, independent corruptions | 256 |
| `g70v5_round_trips` | `app/file/codec_tests.rs` | g70v5 parses back | Round trip | 256 |
| `g70v6_round_trips_and_rejects_bad_encodings` | `app/file/codec_tests.rs` | g70v6 the same as g70v4 (test-only writer) | Round trip, independent corruptions | 256 |
| `g70v7_round_trips_and_rejects_bad_encodings` | `app/file/codec_tests.rs` | g70v7 the same as g70v3 | Round trip, independent corruptions | 256 |
| `g70v8_round_trips_and_rejects_bad_encodings` | `app/file/codec_tests.rs` | g70v8 the same as g70v4 (test-only writer) | Round trip, independent corruptions | 256 |

### Link, transport and I/O

| Property | File | Claim | Oracle | Cases |
|---|---|---|---|---|
| `stream_reader_reads_the_written_frames_however_they_arrive` | `link/reader.rs` | Any split into reads gives what one parse of the whole stream gives; Close and Discard on one changed byte | Round trip for clean frames, independent errors, differential for Discard | 256 |
| `frames_follow_the_one_owner_secondary_state_model` | `link/layer.rs` | Delivery, replies and ignore reasons follow the one-owner secondary state model | Independent model | 128 |
| `assembler_follows_the_model` | `transport/real/assembler.rs` | Assembly outcomes, discard reasons and the waiting fragment follow the reception rules and documented extensions | Independent model | 128 |
| `reader_delivers_what_the_writer_wrote` | `transport/real/mod.rs` | The transport reader delivers what the writer wrote after repeats, drops and another source's series | Independent delivery model and literal segment headers | 128 |
| `tries_endpoints_in_order_and_backs_off_like_the_model` | `tcp/connector.rs` | Endpoint order, connect options, exponential backoff and its reset | Independent model | 256 |
| `a_filter_matches_the_canonical_addresses_it_names` | `tcp/outstation/address_filter.rs` | Address filters match like a model, IPv4 and IPv4-mapped alike | Independent model | 256 |
| `filters_conflict_when_an_address_matches_both` | `tcp/outstation/address_filter.rs` | Two filters conflict, in both orders, exactly when an address matches both | Independent (search with the model) | 256 |
| `a_reply_resumes_where_a_cancelled_write_stopped` | `phys/mod.rs` | A cancelled reply write resumes without losing or repeating bytes, and flushes after the last | Independent (consistency check) | 256 |
| `a_write_follows_the_whole_reply` | `phys/mod.rs` | A write after cancelled reply attempts sends the rest of the reply, then its data | Independent | 128 |

### Master

| Property | File | Claim | Oracle | Cases |
|---|---|---|---|---|
| `next_task_follows_the_scheduling_model` | `master/association/scheduler_tests.rs` | The next task and sleep deadline follow the scheduling guide, within a fairness bound | Independent model | 256 |
| `read_tasks_match_the_model` | `master/tests/task_model.rs` | A read or integrity scan against generated answers and noise: requests, callbacks and how it ends | Independent model | 128 |
| `non_read_tasks_match_the_model` | `master/tests/task_model.rs` | The same for controls, restart, writes and single-request file tasks | Independent model | 64 |
| `relative_time_follows_the_cto_model` | `master/relative_time_tests.rs` | Which responses with relative times are processed, each time, and what extraction reports | Independent model | 512 |
| `read_requests_parse_to_the_headers_written` | `master/request.rs` | READ request bytes, and which headers the outstation accepts | Independent (hand encoding and READ table) | 512 |
| `headers_are_written_in_order` | `master/request.rs` | Generic request headers are written in order, g50v2 times included | Independent (hand encoding) | 512 |
| `commands_are_written_and_parsed_back` | `master/tests/command_headers.rs` | Command grouping and bytes for every command type and index width | Independent (hand encoding) | 512 |
| `echo_matches_only_when_unchanged` | `master/tests/command_headers.rs` | A command echo matches only when nothing changed | Independent | 512 |
| `binary_inputs_are_extracted_as_written` | `master/write_extract_tests.rs` | What extraction reports equals a projection of what the outstation wrote, for each READ variation | Independent projection | 64 |
| `double_bit_binary_inputs_are_extracted_as_written` | `master/write_extract_tests.rs` | The same for double-bit inputs | Independent projection | 64 |
| `binary_output_statuses_are_extracted_as_written` | `master/write_extract_tests.rs` | The same for binary output statuses | Independent projection | 64 |
| `counters_are_extracted_as_written` | `master/write_extract_tests.rs` | The same for counters | Independent projection | 64 |
| `frozen_counters_are_extracted_as_written` | `master/write_extract_tests.rs` | The same for frozen counters | Independent projection | 64 |
| `analog_inputs_are_extracted_as_written` | `master/write_extract_tests.rs` | The same for analog inputs | Independent projection (shares the analog model) | 64 |
| `analog_output_statuses_are_extracted_as_written` | `master/write_extract_tests.rs` | The same for analog output statuses | Independent projection (shares the analog model) | 64 |
| `octet_strings_are_extracted_as_written` | `master/write_extract_tests.rs` | The same for octet strings | Independent projection | 64 |
| `analog_input_deadbands_are_extracted_as_written` | `master/write_extract_tests.rs` | The same for analog input deadbands | Independent projection | 64 |

### Outstation

| Property | File | Claim | Oracle | Cases |
|---|---|---|---|---|
| `behaves_like_a_bounded_queue` | `outstation/database/details/event/list.rs` | The event list keeps insertion order, its ends and size, and `remove_all`'s call order | Independent model | 256 |
| `behaves_like_a_list_with_per_type_limits` | `outstation/database/details/event/tests.rs` | The event buffer's counts, classes, overflow, selection, written bytes and cleared ids | Independent model with its own encoder | 256 |
| `float_deadband_is_symmetric_and_never_exceeded_by_the_same_value` | `outstation/database/details/range/static_db.rs` | The float deadband is symmetric and a value never exceeds it against itself | Relation | 512 |
| `a_larger_float_deadband_never_adds_an_event` | `outstation/database/details/range/static_db.rs` | A larger float deadband never reports a change a smaller one doesn't | Relation | 512 |
| `float_deadband_limits_are_exact` | `outstation/database/details/range/static_db.rs` | Zero, infinite and boundary float deadbands | Independent, except the boundary distance | 512 |
| `analog_event_when_flags_differ_or_the_deadband_is_exceeded` | `outstation/database/details/range/static_db.rs` | An analog event is a change of flags or an exceeded deadband | Independent for flags; the deadband part uses the production check | 512 |
| `integer_deadband_matches_the_distance` | `outstation/database/details/range/static_db.rs` | The integer deadband is exceeded exactly when the distance is larger | Independent (u64 arithmetic) | 512 |
| `fragmenting_does_not_change_what_the_master_sees` | `outstation/database/fragment_tests.rs` | A READ response written in fragments reports what one large fragment reports, items in order, events with their times | Differential for values; independent for items, order and times | 128 |
| `select_and_operate_follow_table_4_9` | `outstation/tests/select_operate_model.rs` | Select-before-operate responses and control callbacks (IEEE 1815-2012 4.4.4.3, Table 4-9) | Independent model | 128 |
| `session_follows_the_event_and_confirm_model` | `outstation/tests/session_model.rs` | Every response, IIN bit and callback for events, confirms, repeats, unsolicited responses and broadcasts | Independent model | 128 |

## Which layer owns what

Overlapping layers assert different things, so an example at a higher layer isn't redundant just
because a property covers the layer below it. Before removing an example, map each of its
assertions to the test that replaces it.

| Overlap | The lower layer checks | The higher layer keeps |
|---|---|---|
| Analog conversion and write/extract | The number and its flags | The variation chosen, missing times, index order and header information |
| Relative time, write/extract and fragmenting | Which CTO relationships are valid, and extraction | Where the writer places CTOs, fragment boundaries, confirms between fragments |
| Command headers, dispatch and select-before-operate | Builder bytes and echo comparison | Callback type, index and status; selection history and timing |
| Attribute codec and attribute WRITEs | Widths, round trip and size | Rejection IIN, the application's approval and the stored values |
| Deadband rules and g34 WRITEs | The event threshold | NaN rejection, negative values, partial success and reading back |
| Event list, event buffer and session | Storage and selection per type | Confirm sequence, UNS bit and deadlines, repeated bytes and callbacks |
| Link state and transport | Role, address and FCB decisions | User data with no transport header, and the link acknowledgment of confirmed data |
| Address filters and server routing | Matching and conflicts | Which registered filter a connection is checked against |

## Known gaps

### Behavior covered only by examples

- **Streaming file reads.** The group 70 codecs are properties, and the task model covers the file
  tasks that take one request. A file read's history over several requests (handle, block numbers,
  the last block, cleanup and the final callback) is checked only by the examples in
  `master/tests/file/read_file.rs`.
- **Stateful attribute-map writes.** The attribute codec is a property; defining, writing and
  reading attributes over a history is checked only by the examples in
  `outstation/tests/attributes.rs`.
- **WRITE-then-READ histories.** The deadband properties check the threshold. WRITEs that mix
  valid and invalid values, followed by READs and updates, are checked only by examples such as
  `outstation/tests/write_dead_band.rs`.
- **The TCP connection sequence.** The connect handler, the scheduler and the outstation session
  each model their own layer. Connecting, reconnecting and the wiring between the transport and
  the sessions are checked by examples (`tcp/client.rs`, `tcp/spawn_tests.rs`,
  `master/tests/reconnect.rs`). A model of the sequence is deliberately deferred.

### Per property

- **Packed, count and ranged sequences:** ranges are 32 indices at most and rarely start at 0;
  only 6-byte objects are counted. Examples cover start 0 and every length.
- **Attributes:** value bytes are checked only by the round trip; the `parses_*` examples pin the
  bytes. Attribute lists aren't generated.
- **Analog conversions:** the integer model is shared with the write/extract properties, so a
  mistake in it would hide from both.
- **Group 70 codecs:** the round trip can't see a layout mistake made on both sides; each object's
  `encodes_known_bytes` pins its layout. Leftover bytes and truncated objects are left to examples.
- **Stream reader:** no bytes inserted or removed between frames, no truncated frames, no
  datagram mode; Discard mode is compared only with a one-shot parse.
- **Link layer:** `Layer::read` isn't exercised, and which ignore reason wins, which is only logged,
  isn't pinned.
- **Transport assembler:** a series that conflicts only in broadcast mode can't be generated.
- **Transport writer and reader:** unconfirmed data from one remote only, no physical addresses or
  broadcasts; the headers of a changed repeat and of the other source's series aren't checked.
- **Connect handler:** the handler only; what the client does with an attempt is left to examples.
- **Address filters:** a small universe of addresses stands in for all of them.
- **Cancelled reply writes:** stream replies only; a pending flush is reached only on the scripted
  stream.
- **Scheduler:** user tasks other than a class scan read, and time synchronization over several
  requests, aren't generated; a rare retry case is pinned by an example.
- **Task model:** time synchronization and directory reads, which take several requests, aren't
  covered; rare outcomes keep their examples in `outcomes.rs` and `unsolicited.rs`.
- **Relative time:** no bytes that fail to parse, no two-byte ranges, no time delays (g52).
- **Request headers:** no group and variation outside the known variations, no attribute headers
  among generic headers, no requests longer than the buffer.
- **Command headers:** the single-header constructors, finishing a header twice and echoes with
  more than one change are left to examples.
- **Write/extract:** READs of all objects only, one fragment, class 1 events only. The times
  reported for some absolute-time variations follow the current API convention rather than a rule
  from the standard.
- **Event list:** panicking closures and `for_each_mut` are left to examples.
- **Event buffer:** no CTO variations, events without time, other octet string lengths or header
  counts past `u16::MAX`.
- **Deadbands:** pairs of special floats come up only in some runs; examples pin them.
- **Fragmenting:** values are compared only against the whole response, and only objects that fit
  an empty fragment are generated.
- **Select-before-operate:** no broadcast controls, no select-before-operate during the confirm
  waits, no responses that don't fit, no control limit.
- **Outstation session:** no controls, malformed or unknown requests, class 0 or static data, other
  event types or link status requests; rare cases are pinned by fixed op lists.

## Measuring a property

A property passing says nothing about how often it reaches the cases it is meant to reach, or
whether it would catch a bug in them. To measure that:

- **Where.** Work in a separate worktree, never in the checkout you develop in, and measure before
  and after a change to the property or its generator.
- **Reach.** Add temporary counters where the model or oracle sees each case, run the property 200
  times at its own budget with fresh seeds, and record in how many runs each counter never fired.
  Don't commit the counters.
- **Mutations.** Hand-pick two or three changes to the code the property claims to check, each a
  plausible bug in a case it should reach. Apply one at a time and run the property alone 20 times,
  each under `timeout 120`; a timeout counts as caught. Report how many of the 20 runs caught it,
  before and after.
- **Settings.** Set `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1` and leave `PROPTEST_CASES` unset, so
  that the property runs at its own budget and a seed saved by an earlier failure, perhaps under
  another mutation, is neither replayed nor written:

  ```text
  PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 cargo test -p dnp3 --lib -- <property>
  ```

- **Restoring sources.** After restoring a mutated file, touch it. A restored copy can carry an
  older timestamp than the mutated build, and cargo then reuses the stale binary.
- **Where results go.** Measured numbers stay with the change that measured them, such as its PR.
  They don't go into module docs or this file, because they date quickly.

Ordinary runs keep proptest's failure persistence: a failing property saves its seed under
`dnp3/proptest-regressions/`, which is ignored by git, and replays it first until the file is
deleted. A failure a property finds becomes a deterministic example test; regression files are
never committed.

## Changing a property

A PR that adds or changes a property updates its module doc (claim, oracle and whether it is
independent, the generator's limits, the cases it is built to reach, and its known gaps) and its
row in this file, including its known gaps above. Neither holds measured numbers.
