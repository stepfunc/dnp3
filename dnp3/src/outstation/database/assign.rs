// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::parser::{HeaderCollection, HeaderIterator};
use crate::app::{FunctionCode, Iin2};
use crate::outstation::database::EventClass;
use crate::outstation::target::{AssignClassHeader, AssignClassTarget, Selector};
use crate::outstation::{PointIndices, PointType};

/// A point header of an ASSIGN_CLASS request, with the class of its assignment set
#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct Assignment {
    pub(crate) point_type: PointType,
    pub(crate) indices: PointIndices,
    pub(crate) class: Option<EventClass>,
}

/// The point headers of an ASSIGN_CLASS request in order, each with the class of the assignment
/// set it belongs to, or the error of a header that isn't a valid assignment
///
/// A class header starts an assignment set (IEEE 1815-2012 4.4.14). A point header before any
/// class header assigns its points to no class (TB2021-001), which also removes them from class
/// 0 and isn't supported: it's an error, IIN2.2.
pub(crate) struct Assignments<'a> {
    headers: HeaderIterator<'a>,
    selector: Selector,
    class: Option<Option<EventClass>>,
}

impl<'a> Assignments<'a> {
    pub(crate) fn new(headers: &HeaderCollection<'a>) -> Self {
        Self {
            headers: headers.iter(),
            selector: Selector::new(FunctionCode::AssignClass),
            class: None,
        }
    }
}

impl Iterator for Assignments<'_> {
    type Item = Result<Assignment, Iin2>;

    fn next(&mut self) -> Option<Self::Item> {
        for header in self.headers.by_ref() {
            match self.selector.select::<AssignClassTarget>(header) {
                Ok(AssignClassHeader::Class(class)) => self.class = Some(class),
                Ok(AssignClassHeader::Points(point_type, indices)) => {
                    let Some(class) = self.class else {
                        tracing::warn!(
                            "ASSIGN_CLASS of points to no class, before any class header, isn't supported"
                        );
                        return Some(Err(Iin2::PARAMETER_ERROR));
                    };
                    return Some(Ok(Assignment {
                        point_type,
                        indices,
                        class,
                    }));
                }
                Err(err) => {
                    self.selector.log();
                    return Some(Err(err));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::parse::options::ParseOptions;

    const CLASS_2: Option<EventClass> = Some(EventClass::Class2);

    fn assignments(zero_length_strings: bool, objects: &[u8]) -> Vec<Result<Assignment, Iin2>> {
        let options = ParseOptions {
            parse_zero_length_strings: zero_length_strings,
        };
        let headers = HeaderCollection::parse(options, FunctionCode::AssignClass, objects).unwrap();
        Assignments::new(&headers).collect()
    }

    fn assignment(
        point_type: PointType,
        indices: PointIndices,
        class: Option<EventClass>,
    ) -> Result<Assignment, Iin2> {
        Ok(Assignment {
            point_type,
            indices,
            class,
        })
    }

    #[test]
    fn each_point_header_takes_the_class_of_its_set() {
        // an empty set first, then class 2 for two headers, an empty set, no events for one, and
        // a last empty set
        let objects = [
            60, 2, 6, 60, 3, 6, 1, 0, 6, 30, 0, 0, 1, 2, 60, 2, 6, 60, 1, 6, 20, 0, 1, 0, 0, 9, 0,
            60, 4, 6,
        ];
        assert_eq!(
            assignments(false, &objects),
            [
                assignment(PointType::BinaryInput, PointIndices::All, CLASS_2),
                assignment(PointType::AnalogInput, PointIndices::Range(1, 2), CLASS_2),
                assignment(PointType::Counter, PointIndices::Range(0, 9), None),
            ]
        );
    }

    #[test]
    fn every_point_type_of_table_4_12_that_has_events() {
        let types = [
            (1, PointType::BinaryInput),
            (3, PointType::DoubleBitBinaryInput),
            (10, PointType::BinaryOutputStatus),
            (20, PointType::Counter),
            (21, PointType::FrozenCounter),
            (30, PointType::AnalogInput),
            (40, PointType::AnalogOutputStatus),
            (110, PointType::OctetString),
        ];
        for (group, point_type) in types {
            assert_eq!(
                assignments(false, &[60, 3, 6, group, 0, 6]),
                [assignment(point_type, PointIndices::All, CLASS_2)],
                "group {group}"
            );
        }
    }

    #[test]
    fn octet_string_ranges_parse_only_with_zero_length_strings() {
        let objects = [60, 3, 6, 110, 0, 0, 4, 5];
        assert_eq!(
            assignments(true, &objects),
            [assignment(
                PointType::OctetString,
                PointIndices::Range(4, 5),
                CLASS_2
            )]
        );
        let options = ParseOptions {
            parse_zero_length_strings: false,
        };
        assert!(HeaderCollection::parse(options, FunctionCode::AssignClass, &objects).is_err());
    }

    #[test]
    fn points_before_any_class_header_are_an_error() {
        assert_eq!(
            assignments(false, &[1, 0, 6, 60, 2, 6]),
            [Err(Iin2::PARAMETER_ERROR)]
        );
    }

    #[test]
    fn headers_assign_class_does_not_support_are_errors() {
        // binary input events, and the static variation with data
        assert_eq!(
            assignments(false, &[60, 2, 6, 2, 0, 6]),
            [Err(Iin2::OBJECT_UNKNOWN)]
        );
        assert_eq!(
            assignments(false, &[60, 2, 6, 1, 2, 6]),
            [Err(Iin2::OBJECT_UNKNOWN)]
        );
    }
}
