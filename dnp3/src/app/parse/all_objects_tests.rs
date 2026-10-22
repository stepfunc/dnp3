//! Pins which variations an all-objects header can name outside a READ request

use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails};
use crate::app::{FunctionCode, ObjectParseError, QualifierCode, Variation};

/// the variations of each group that an all-objects header can name, outside group 0, where
/// every variation can be named
const ACCEPTED: &[(u8, &[u8])] = &[
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
    (110, &[0]),
    (111, &[0]),
];

fn accepted(group: u8, var: u8) -> bool {
    group == 0
        || ACCEPTED
            .iter()
            .any(|(g, vars)| *g == group && vars.contains(&var))
}

#[test]
fn all_objects_headers_outside_read_accept_these_variations() {
    for function in [
        FunctionCode::Response,
        FunctionCode::ImmediateFreeze,
        FunctionCode::EnableUnsolicited,
    ] {
        for group in 0..=u8::MAX {
            for var in 0..=u8::MAX {
                let Some(variation) = Variation::lookup(group, var) else {
                    continue;
                };
                let input = [group, var, QualifierCode::AllObjects.as_u8()];
                let result = HeaderCollection::parse(ParseOptions::default(), function, &input)
                    .map(|headers| headers.get_only_header().unwrap());
                if accepted(group, var) {
                    let header = result.unwrap();
                    assert_eq!(header.variation, variation);
                    assert!(
                        matches!(header.details, HeaderDetails::AllObjects(_)),
                        "{function:?} g{group}v{var}"
                    );
                } else {
                    assert_eq!(
                        result.err(),
                        Some(ObjectParseError::InvalidQualifierForVariation(
                            variation,
                            QualifierCode::AllObjects
                        )),
                        "{function:?} g{group}v{var}"
                    );
                }
            }
        }
    }
}
