// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Conversions from `f64`, which only `as` provides. To an integer, `as` truncates toward zero,
//! saturates at the integer's bounds, and turns NaN into zero
#![expect(
    clippy::as_conversions,
    reason = "as is the only conversion from a float to an integer or to f32"
)]

pub(crate) fn f64_to_i16(value: f64) -> i16 {
    value as i16
}

pub(crate) fn f64_to_i32(value: f64) -> i32 {
    value as i32
}

pub(crate) fn f64_to_u16(value: f64) -> u16 {
    value as u16
}

pub(crate) fn f64_to_u32(value: f64) -> u32 {
    value as u32
}

/// rounds to the nearest `f32`, and saturates to infinity past its range
pub(crate) fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_conversions_truncate_and_saturate() {
        assert_eq!(f64_to_i16(-1.9), -1);
        assert_eq!(f64_to_i16(1e9), i16::MAX);
        assert_eq!(f64_to_i32(-1e12), i32::MIN);
        assert_eq!(f64_to_u16(-5.0), 0);
        assert_eq!(f64_to_u32(f64::INFINITY), u32::MAX);
        assert_eq!(f64_to_u16(f64::NAN), 0);
    }

    #[test]
    fn f32_conversion_rounds_and_saturates() {
        assert_eq!(f64_to_f32(0.5).to_bits(), 0.5f32.to_bits());
        assert_eq!(f64_to_f32(f64::MAX).to_bits(), f32::INFINITY.to_bits());
    }
}
