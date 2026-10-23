// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Pins the identity of object headers: which variations each qualifier accepts, in a READ request
//! and outside one, the variation and qualifier of each parsed header, and the error for each
//! header the parser rejects or that is truncated
//!
//! Each header names one object (a range of index 0, or a count of 1) and its payload is found by
//! trying zero-filled payloads of increasing length, so the probe needs no table of object sizes.
//! Attributes and file objects, which zeros don't encode, get a valid encoding instead.

use crate::app::attr::AttrParseError;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::{HeaderCollection, ObjectHeader};
use crate::app::{FunctionCode, ObjectParseError, QualifierCode, Variation};

use ObjectParseError as E;
use QualifierCode as Q;

/// The qualifiers of headers outside a READ
const QUALIFIERS: [QualifierCode; 8] = [
    Q::AllObjects,
    Q::Range8,
    Q::Range16,
    Q::Count8,
    Q::Count16,
    Q::CountAndPrefix8,
    Q::CountAndPrefix16,
    Q::FreeFormat16,
];

/// A response and requests other than READ, which all parse headers alike
const FUNCTIONS: [FunctionCode; 3] = [
    FunctionCode::Response,
    FunctionCode::Write,
    FunctionCode::DirectOperate,
];

/// Requests whose all-objects headers the probes also check
const ALL_OBJECTS_FUNCTIONS: [FunctionCode; 2] = [
    FunctionCode::ImmediateFreeze,
    FunctionCode::EnableUnsolicited,
];

/// The function and qualifier of each header that the probes check
fn probed() -> impl Iterator<Item = (FunctionCode, QualifierCode)> {
    FUNCTIONS
        .into_iter()
        .flat_map(|function| QUALIFIERS.map(|qualifier| (function, qualifier)))
        .chain(ALL_OBJECTS_FUNCTIONS.map(|function| (function, Q::AllObjects)))
}

/// The longest zero-filled payload a probe tries: an index of 2 bytes and a 255 byte octet string
const MAX_ZEROS: usize = 257;

/// The header bytes before the payload: group, variation, qualifier, and the range or count of a
/// single object
fn prefix(variation: Variation, qualifier: QualifierCode) -> Vec<u8> {
    let (group, var) = variation.to_group_and_var();
    let mut bytes = vec![group, var, qualifier.as_u8()];
    let rest: &[u8] = match qualifier {
        Q::AllObjects | Q::FreeFormat16 => &[],
        Q::Range8 => &[0, 0],
        Q::Range16 => &[0, 0, 0, 0],
        Q::Count8 | Q::CountAndPrefix8 => &[1],
        Q::Count16 | Q::CountAndPrefix16 => &[1, 0],
    };
    bytes.extend_from_slice(rest);
    bytes
}

/// A visible string attribute of one character
const ATTR: [u8; 3] = [1, 1, b'A'];

/// A valid object of each file variation, from its fixed fields and its strings
fn file_object(variation: Variation) -> Option<Vec<u8>> {
    let (fixed, strings): (&[u8], &[u8]) = match variation {
        // user name offset and size, password offset and size, authentication key
        Variation::Group70Var2 => (&[12, 0, 1, 0, 13, 0, 1, 0, 0, 0, 0, 0], b"up"),
        // file name offset and size, then 22 bytes of time of creation, permissions,
        // authentication key, file size, operational mode, maximum block size and request id
        Variation::Group70Var3 => (&[26, 0, 1, 0], b"f"),
        // file handle, file size, maximum block size, request id and status, without text
        Variation::Group70Var4 => (&[0; 13], b""),
        // file handle and block number, without data
        Variation::Group70Var5 => (&[0; 8], b""),
        // file handle, block number and status, without text
        Variation::Group70Var6 => (&[0; 9], b""),
        // file name offset and size, then 16 bytes of file type, file size, time of creation,
        // permissions and request id
        Variation::Group70Var7 => (&[20, 0, 1, 0], b"f"),
        // file specification
        Variation::Group70Var8 => (&[], b"f"),
        _ => return None,
    };
    let zeros = match variation {
        Variation::Group70Var3 => 22,
        Variation::Group70Var7 => 16,
        _ => 0,
    };
    Some([fixed, &vec![0; zeros], strings].concat())
}

/// The payloads a probe tries, in order
fn payloads(variation: Variation, qualifier: QualifierCode) -> Vec<Vec<u8>> {
    if let Variation::Group0(_) = variation {
        let index: Option<&[u8]> = match qualifier {
            Q::Range8 | Q::Range16 => Some(&[]),
            Q::CountAndPrefix8 => Some(&[0]),
            Q::CountAndPrefix16 => Some(&[0, 0]),
            _ => None,
        };
        if let Some(index) = index {
            return vec![[index, &ATTR].concat()];
        }
    }
    if qualifier == Q::FreeFormat16 {
        if let Some(object) = file_object(variation) {
            return vec![free_format(&object)];
        }
        return (0..=MAX_ZEROS).map(|n| free_format(&vec![0; n])).collect();
    }
    (0..=MAX_ZEROS).map(|n| vec![0; n]).collect()
}

/// The free-format payload of one object
fn free_format(object: &[u8]) -> Vec<u8> {
    let len = u16::try_from(object.len()).unwrap().to_le_bytes();
    [&[1], &len[..], object].concat()
}

/// What a probe found for a variation and qualifier
#[derive(Clone, Debug, PartialEq)]
enum Probe {
    /// these bytes encode a header that the parser accepts
    Accepted(Vec<u8>),
    /// the parser rejects the header with this error
    Rejected(ObjectParseError),
}

/// Find an encoding of one header that the parser accepts, or the error that rejects it
///
/// A payload that is too short gives `InsufficientBytes`; any other error, or extra bytes that
/// fail to parse as a second header, ends the search.
fn probe(
    options: ParseOptions,
    function: FunctionCode,
    variation: Variation,
    qualifier: QualifierCode,
) -> Probe {
    let mut last = E::InsufficientBytes;
    for payload in payloads(variation, qualifier) {
        let bytes = [prefix(variation, qualifier), payload].concat();
        match HeaderCollection::parse(options, function, &bytes) {
            Ok(headers) => {
                assert_eq!(headers.iter().count(), 1);
                return Probe::Accepted(bytes);
            }
            Err(E::InsufficientBytes) => continue,
            Err(err) => {
                last = err;
                break;
            }
        }
    }
    Probe::Rejected(last)
}

/// The variations of each group, outside groups 0, 110 and 111, that an all-objects header accepts,
/// in a READ or outside one
const ALL_OBJECTS: &[(u8, &[u8])] = &[
    (1, &[0, 1, 2]),
    (2, &[0, 1, 2, 3]),
    (3, &[0, 1, 2]),
    (4, &[0, 1, 2, 3]),
    (10, &[0, 1, 2]),
    (11, &[0, 1, 2]),
    (13, &[1, 2]),
    (20, &[0, 1, 2, 5, 6]),
    (21, &[0, 1, 2, 5, 6, 9, 10]),
    (22, &[0, 1, 2, 5, 6]),
    (23, &[0, 1, 2, 5, 6]),
    (30, &[0, 1, 2, 3, 4, 5, 6]),
    (31, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    (32, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    (33, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    (34, &[0, 1, 2, 3]),
    (40, &[0, 1, 2, 3, 4]),
    (42, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    (43, &[1, 2, 3, 4, 5, 6, 7, 8]),
    (60, &[1, 2, 3, 4]),
    (80, &[1]),
    (102, &[0, 1]),
];

/// The same for a range header of either width, in a READ or outside one
const RANGE: &[(u8, &[u8])] = &[
    (1, &[0, 1, 2]),
    (3, &[0, 1, 2]),
    (10, &[0, 1, 2]),
    (20, &[0, 1, 2, 5, 6]),
    (21, &[0, 1, 2, 5, 6, 9, 10]),
    (30, &[0, 1, 2, 3, 4, 5, 6]),
    (31, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    (34, &[0, 1, 2, 3]),
    (40, &[0, 1, 2, 3, 4]),
    (80, &[1]),
    (102, &[0, 1]),
];

/// The same for a count header of either width outside a READ
const COUNT: &[(u8, &[u8])] = &[(50, &[1, 2, 3, 4]), (51, &[1, 2]), (52, &[1, 2])];

/// The variations that a READ counts but that have no objects to follow a count outside a READ:
/// the "any variation" of each event group, the event classes and octet string events
pub(crate) const DATALESS_COUNT_VARIATIONS: [Variation; 12] = [
    Variation::Group2Var0,
    Variation::Group4Var0,
    Variation::Group11Var0,
    Variation::Group22Var0,
    Variation::Group23Var0,
    Variation::Group32Var0,
    Variation::Group33Var0,
    Variation::Group42Var0,
    Variation::Group60Var2,
    Variation::Group60Var3,
    Variation::Group60Var4,
    Variation::Group111(0),
];

/// The same for a count and prefix header of either width
const PREFIXED: &[(u8, &[u8])] = &[
    (2, &[1, 2, 3]),
    (4, &[1, 2, 3]),
    (11, &[1, 2]),
    (12, &[1]),
    (13, &[1, 2]),
    (22, &[1, 2, 5, 6]),
    (23, &[1, 2, 5, 6]),
    (32, &[1, 2, 3, 4, 5, 6, 7, 8]),
    (33, &[1, 2, 3, 4, 5, 6, 7, 8]),
    (34, &[1, 2, 3]),
    (41, &[1, 2, 3, 4]),
    (42, &[1, 2, 3, 4, 5, 6, 7, 8]),
    (43, &[1, 2, 3, 4, 5, 6, 7, 8]),
];

/// The same for a free-format header
const FREE_FORMAT: &[(u8, &[u8])] = &[(70, &[2, 3, 4, 5, 6, 7, 8])];

fn table(qualifier: QualifierCode) -> &'static [(u8, &'static [u8])] {
    match qualifier {
        Q::AllObjects => ALL_OBJECTS,
        Q::Range8 | Q::Range16 => RANGE,
        Q::Count8 | Q::Count16 => COUNT,
        Q::CountAndPrefix8 | Q::CountAndPrefix16 => PREFIXED,
        Q::FreeFormat16 => FREE_FORMAT,
    }
}

/// What the parser does with a header of one object of the variation and qualifier
fn expected(
    options: ParseOptions,
    variation: Variation,
    qualifier: QualifierCode,
) -> Result<(), ObjectParseError> {
    let (group, var) = variation.to_group_and_var();
    let zero_length = options.parse_zero_length_strings;
    let accepted = match (qualifier, group) {
        (Q::AllObjects, 0) => true,
        (Q::AllObjects, 110 | 111) => var == 0,
        (Q::Range8 | Q::Range16, 0) => true,
        (Q::Range8 | Q::Range16, 110) if var == 0 && !zero_length => {
            return Err(E::ZeroLengthOctetData)
        }
        (Q::Range8 | Q::Range16, 110) => true,
        (Q::CountAndPrefix8 | Q::CountAndPrefix16, 0) => var != 254,
        (Q::CountAndPrefix8 | Q::CountAndPrefix16, 111) if var == 0 && !zero_length => {
            return Err(E::ZeroLengthOctetData)
        }
        (Q::CountAndPrefix8 | Q::CountAndPrefix16, 111) => true,
        (_, 0 | 110 | 111) => false,
        _ => table(qualifier)
            .iter()
            .any(|(g, vars)| *g == group && vars.contains(&var)),
    };
    if accepted {
        Ok(())
    } else {
        Err(E::InvalidQualifierForVariation(variation, qualifier))
    }
}

const OPTIONS: [ParseOptions; 2] = [
    ParseOptions {
        parse_zero_length_strings: false,
    },
    ParseOptions {
        parse_zero_length_strings: true,
    },
];

/// Every header the parser accepts outside a READ, with the variation and qualifier it encodes
pub(crate) fn accepted_headers(
    options: ParseOptions,
    function: FunctionCode,
) -> Vec<(Variation, QualifierCode, Vec<u8>)> {
    let mut headers = Vec::new();
    for qualifier in QUALIFIERS {
        for variation in Variation::all() {
            if let Probe::Accepted(bytes) = probe(options, function, variation, qualifier) {
                headers.push((variation, qualifier, bytes));
            }
        }
    }
    headers
}

#[test]
fn each_qualifier_accepts_these_variations_outside_read() {
    for options in OPTIONS {
        for (function, qualifier) in probed() {
            for variation in Variation::all() {
                let actual = match probe(options, function, variation, qualifier) {
                    Probe::Accepted(_) => Ok(()),
                    Probe::Rejected(err) => Err(err),
                };
                assert_eq!(
                    actual,
                    expected(options, variation, qualifier),
                    "{options:?} {function:?} {variation:?} {qualifier:?}"
                );
            }
        }
    }
}

#[test]
fn each_parsed_header_carries_its_variation_and_qualifier() {
    for options in OPTIONS {
        for (function, qualifier) in probed() {
            for variation in Variation::all() {
                let Probe::Accepted(bytes) = probe(options, function, variation, qualifier) else {
                    continue;
                };
                let headers = HeaderCollection::parse(options, function, &bytes).unwrap();
                let header = headers.get_only_header().unwrap();
                assert_eq!(header.variation(), variation);
                assert_eq!(header.qualifier(), qualifier);
                assert!(!matches!(header, ObjectHeader::Read(..)));
                assert_eq!(
                    header.count().is_some(),
                    matches!(qualifier, Q::Count8 | Q::Count16)
                );
            }
        }
    }
}

#[test]
fn headers_in_one_fragment_each_carry_their_variation_and_qualifier() {
    for options in OPTIONS {
        for function in FUNCTIONS {
            let accepted = accepted_headers(options, function);
            let fragment: Vec<u8> = accepted
                .iter()
                .flat_map(|(_, _, bytes)| bytes.clone())
                .collect();
            let headers = HeaderCollection::parse(options, function, &fragment).unwrap();
            let actual: Vec<(Variation, QualifierCode)> = headers
                .iter()
                .map(|header| (header.variation(), header.qualifier()))
                .collect();
            let expected: Vec<(Variation, QualifierCode)> =
                accepted.iter().map(|(v, q, _)| (*v, *q)).collect();
            assert_eq!(actual, expected);
        }
    }
}

/// The same for a count header of either width in a READ, which has no objects to follow the count
const READ_COUNT: &[(u8, &[u8])] = &[
    (2, &[0, 1, 2, 3]),
    (4, &[0, 1, 2, 3]),
    (11, &[0, 1, 2]),
    (13, &[1, 2]),
    (22, &[0, 1, 2, 5, 6]),
    (23, &[0, 1, 2, 5, 6]),
    (32, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    (33, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    (42, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    (43, &[1, 2, 3, 4, 5, 6, 7, 8]),
    (50, &[1, 2, 3, 4]),
    (51, &[1, 2]),
    (52, &[1, 2]),
    (60, &[2, 3, 4]),
];

/// The kinds of qualifier in a READ, which accept different variations
#[derive(Clone, Copy, Debug)]
pub(crate) enum ReadKind {
    AllObjects,
    Range,
    Count,
}

/// The qualifiers of headers in a READ, with their kind
const READ_QUALIFIERS: [(QualifierCode, ReadKind); 5] = [
    (Q::AllObjects, ReadKind::AllObjects),
    (Q::Range8, ReadKind::Range),
    (Q::Range16, ReadKind::Range),
    (Q::Count8, ReadKind::Count),
    (Q::Count16, ReadKind::Count),
];

/// true if a READ accepts the variation with this kind of qualifier, written out independently of
/// the parser
pub(crate) fn read_accepts(kind: ReadKind, variation: Variation) -> bool {
    let (group, var) = variation.to_group_and_var();
    let table = match (kind, group) {
        (ReadKind::Count, 0) => return false,
        (_, 0) => return true,
        (ReadKind::AllObjects, 110 | 111) | (ReadKind::Range, 110) => return var == 0,
        (ReadKind::Count, 111) => return true,
        (_, 110 | 111) => return false,
        (ReadKind::AllObjects, _) => ALL_OBJECTS,
        (ReadKind::Range, _) => RANGE,
        (ReadKind::Count, _) => READ_COUNT,
    };
    table
        .iter()
        .any(|(g, vars)| *g == group && vars.contains(&var))
}

#[test]
fn each_read_qualifier_accepts_these_variations() {
    for options in OPTIONS {
        for (qualifier, kind) in READ_QUALIFIERS {
            for variation in Variation::all() {
                let bytes = prefix(variation, qualifier);
                let actual = HeaderCollection::parse(options, FunctionCode::Read, &bytes)
                    .map(|headers| headers.iter().count());
                let expected = if read_accepts(kind, variation) {
                    Ok(1)
                } else {
                    Err(E::InvalidQualifierForVariation(variation, qualifier))
                };
                assert_eq!(actual, expected, "{options:?} {variation:?} {qualifier:?}");
            }
        }
    }
}

#[test]
fn each_read_header_carries_its_variation_and_qualifier() {
    let qualifiers = [Q::AllObjects, Q::Range8, Q::Range16, Q::Count8, Q::Count16];
    let mut accepted = 0;
    for qualifier in qualifiers {
        for variation in Variation::all() {
            let bytes = prefix(variation, qualifier);
            let parsed =
                HeaderCollection::parse(ParseOptions::default(), FunctionCode::Read, &bytes);
            if let Ok(headers) = parsed {
                let header = headers.get_only_header().unwrap();
                assert_eq!(header.variation(), variation);
                assert_eq!(header.qualifier(), qualifier);
                assert!(header.count().is_none());
                accepted += 1;
            }
        }
    }
    // the number of variation and qualifier pairs that a READ accepts, counting each variation
    // of groups 0, 110 and 111
    assert_eq!(accepted, 1628);
}

#[test]
fn truncated_headers_are_rejected() {
    for options in OPTIONS {
        for function in FUNCTIONS {
            for (variation, qualifier, bytes) in accepted_headers(options, function) {
                for len in 1..bytes.len() {
                    // a cut inside the value of an attribute is an attribute error
                    let in_attribute = matches!(variation, Variation::Group0(_))
                        && qualifier != Q::AllObjects
                        && len >= bytes.len() - ATTR.len();
                    let expected = if in_attribute {
                        E::BadAttribute(AttrParseError::ReadError)
                    } else {
                        E::InsufficientBytes
                    };
                    assert_eq!(
                        HeaderCollection::parse(options, function, &bytes[..len]).err(),
                        Some(expected),
                        "{variation:?} {qualifier:?} {len}"
                    );
                }
            }
        }
    }
}

#[test]
fn unknown_variations_and_qualifiers_are_rejected() {
    for function in FUNCTIONS {
        for group in 0..=u8::MAX {
            for var in 0..=u8::MAX {
                if Variation::lookup(group, var).is_none() {
                    let input = [group, var, Q::AllObjects.as_u8()];
                    assert_eq!(
                        HeaderCollection::parse(ParseOptions::default(), function, &input).err(),
                        Some(E::UnknownGroupVariation(group, var))
                    );
                }
            }
        }
        for qualifier in 0..=u8::MAX {
            if QualifierCode::from(qualifier).is_none() {
                let input = [1, 2, qualifier, 0, 0, 0];
                assert_eq!(
                    HeaderCollection::parse(ParseOptions::default(), function, &input).err(),
                    Some(E::UnknownQualifier(qualifier))
                );
            }
        }
    }
}

#[test]
fn ranges_with_start_after_stop_are_rejected_for_every_variation() {
    for function in FUNCTIONS {
        for variation in Variation::all() {
            let (group, var) = variation.to_group_and_var();
            let cases: [(&[u8], ObjectParseError); 2] = [
                (
                    &[group, var, Q::Range8.as_u8(), 2, 1],
                    E::InvalidRange(2, 1),
                ),
                (
                    &[group, var, Q::Range16.as_u8(), 2, 1, 1, 1],
                    E::InvalidRange(0x0102, 0x0101),
                ),
            ];
            for (input, err) in cases {
                assert_eq!(
                    HeaderCollection::parse(ParseOptions::default(), function, input).err(),
                    Some(err)
                );
            }
        }
    }
}

#[test]
fn free_format_headers_need_a_count_of_one() {
    for function in FUNCTIONS {
        for var in 2..=8 {
            for count in [0, 2, u8::MAX] {
                let input = [70, var, Q::FreeFormat16.as_u8(), count, 0, 0];
                assert_eq!(
                    HeaderCollection::parse(ParseOptions::default(), function, &input).err(),
                    Some(E::UnsupportedFreeFormatCount(count))
                );
            }
        }
    }
}
