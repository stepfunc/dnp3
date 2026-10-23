// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

pub use app_enums::*;
pub use buffer_size::*;
pub use file::*;
pub use header::*;
pub use listener::*;
pub use maybe_async::MaybeAsync;
pub use parse_error::*;
pub use retry::*;
pub use sequence::*;
pub use shutdown::*;
pub use timeout::*;
pub use types::*;
pub use variations::Variation;

/// Types used for making binary and analog output control requests
pub mod control {
    pub use super::control_enums::*;
    pub use super::control_types::ControlCode;
    pub use super::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
}

/// Types association with Device Attributes (Group 0)
pub mod attr;
mod control_types;
pub(crate) use control_types::CommandObject;
#[rustfmt::skip]
mod app_enums;
mod buffer_size;
mod control_enums;
#[cfg(test)]
mod enum_tests;
mod extensions;

pub(crate) mod file;
mod header;
mod listener;
mod maybe_async;
/// Measurement types, e.g. Binary, Analog, Counter, etc
pub mod measurement;
pub(crate) mod parse;
mod retry;
mod sequence;
mod shutdown;
mod timeout;
mod types;

#[rustfmt::skip]
pub(crate) mod variations;

pub(crate) mod format;
mod parse_error;

/// Global options for the application layer. These are workarounds
/// that allow us to tweak behaviors, e.g., parsing 0-length strings,
/// in a global manner. In a future 2.0, these options will be configurable
/// on individual master and outstation sessions.
pub mod options;

#[rustfmt::skip]
pub(crate) mod gen {
    pub(crate) mod all;
    pub(crate) mod conversion;
    pub(crate) mod count;
    pub(crate) mod prefixed;
    pub(crate) mod ranged;
    pub(crate) mod read;
}

#[cfg(test)]
impl Variation {
    /// every variation that `lookup` produces, in group and variation order
    pub(crate) fn all() -> impl Iterator<Item = Variation> {
        (0..=u8::MAX)
            .flat_map(|group| (0..=u8::MAX).filter_map(move |var| Variation::lookup(group, var)))
    }
}

#[cfg(test)]
mod test {
    use super::Variation;

    #[test]
    fn variation_lookup_round_trips_through_group_and_var() {
        let mut count = 0;
        for group in 0..=255 {
            for var in 0..=255 {
                if let Some(x) = Variation::lookup(group, var) {
                    assert_eq!(x.to_group_and_var(), (group, var), "{x:?}");
                    count += 1;
                }
            }
        }
        assert!(count > 0);
    }

    #[test]
    fn group0_var254_has_two_representations() {
        // lookup only produces Group0Var254, but Group0(254) encodes the same
        assert_eq!(Variation::lookup(0, 254), Some(Variation::Group0Var254));
        assert_eq!(Variation::Group0Var254.to_group_and_var(), (0, 254));
        assert_eq!(Variation::Group0(254).to_group_and_var(), (0, 254));
    }
}
