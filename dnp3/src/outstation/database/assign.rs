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
