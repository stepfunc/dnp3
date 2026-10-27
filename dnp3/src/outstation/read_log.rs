// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::attr::BadAttribute;
use crate::app::{QualifierCode, Variation};
use crate::outstation::database::details::attrs;
use crate::outstation::database::details::range::static_db::IndexRange;
use crate::outstation::session::deferred;
use crate::outstation::target;
use crate::util::skipped::Count;

/// The diagnostics that selecting and writing the response to a READ can log
///
/// Each method takes the typed values of one diagnostic, so that an implementation that logs
/// nothing formats nothing. [`Tracing`] logs each one from the module that raises it.
pub(crate) trait ReadLog: Copy {
    /// The first header of the request that READ doesn't support, and how many
    fn unsupported_headers(self, header: (Variation, QualifierCode), count: Count);
    /// An attribute selection that exceeds the maximum per request
    fn attr_selection_full(self, max: usize);
    /// An attribute variation that can't be read with the all objects qualifier
    fn attr_variation_with_all_objects(self, variation: u8);
    /// An attribute READ with a range that isn't one attribute set
    fn attr_range_unsupported(self, range: IndexRange);
    /// An attribute that can't be encoded, which the response skips
    fn attr_not_encodable(self, err: BadAttribute);
    /// The first header of a deferred READ that doesn't fit its queue, and how many
    fn deferred_capacity(self, max: usize, header: (Variation, QualifierCode), count: Count);
}

/// Logs each diagnostic with `tracing`
#[derive(Copy, Clone)]
pub(crate) struct Tracing;

impl ReadLog for Tracing {
    fn unsupported_headers(self, header: (Variation, QualifierCode), count: Count) {
        target::log_unsupported(crate::app::FunctionCode::Read, header, count);
    }

    fn attr_selection_full(self, max: usize) {
        attrs::log_selection_full(max);
    }

    fn attr_variation_with_all_objects(self, variation: u8) {
        attrs::log_variation_with_all_objects(variation);
    }

    fn attr_range_unsupported(self, range: IndexRange) {
        attrs::log_range_unsupported(range);
    }

    fn attr_not_encodable(self, err: BadAttribute) {
        attrs::log_not_encodable(err);
    }

    fn deferred_capacity(self, max: usize, header: (Variation, QualifierCode), count: Count) {
        deferred::log_capacity(max, header, count);
    }
}
