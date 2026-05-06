//! Fixed-point decimal type.

use super::error::TypeError;
use alloc::format;
use alloc::string::ToString;
use core::fmt;

/// Fixed-point decimal: `real = unscaled × 10^−scale`.
///
/// Scale is bounded to [`Decimal::MAX_SCALE`] (38), matching the maximum
/// precision of SQL `NUMERIC` and most fixed-point decimal standards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Decimal {
    pub unscaled: i128,
    pub scale: u32,
}

impl Decimal {
    /// Maximum scale (38), matching SQL NUMERIC maximum precision.
    pub const MAX_SCALE: u32 = 38;

    pub const fn try_new(unscaled: i128, scale: u32) -> Result<Self, TypeError> {
        if scale > Self::MAX_SCALE {
            return Err(TypeError::DecimalScaleTooLarge { scale });
        }
        Ok(Self { unscaled, scale })
    }

    pub const fn new_unchecked(unscaled: i128, scale: u32) -> Self {
        Self { unscaled, scale }
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale == 0 {
            return write!(f, "{}", self.unscaled);
        }
        let negative = self.unscaled < 0;
        let abs = self.unscaled.unsigned_abs();
        let scale = self.scale as usize;
        let mut digits = abs.to_string();
        if digits.len() <= scale {
            digits = format!("{:0>width$}", digits, width = scale + 1);
        }
        let split = digits.len() - scale;
        let (int_part, frac_part) = digits.split_at(split);
        if negative {
            write!(f, "-{int_part}.{frac_part}")
        } else {
            write!(f, "{int_part}.{frac_part}")
        }
    }
}
