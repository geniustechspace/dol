//! [`LiteralRange`]: a bound-pair for [`super::Literal`].

use alloc::boxed::Box;
use core::fmt;
use core::ops::Bound;

use super::Literal;
use crate::value::fmt_bound;

/// A range bound-pair for [`Literal`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct LiteralRange<'a> {
    pub start: Bound<Box<Literal<'a>>>,
    pub end: Bound<Box<Literal<'a>>>,
}

impl<'a> LiteralRange<'a> {
    pub fn new(start: Bound<Literal<'a>>, end: Bound<Literal<'a>>) -> Self {
        Self {
            start: start.map(Box::new),
            end: end.map(Box::new),
        }
    }

    pub fn unbounded() -> Self {
        Self {
            start: Bound::Unbounded,
            end: Bound::Unbounded,
        }
    }

    /// Convert any borrowed data to owned, erasing the lifetime.
    pub fn into_static(self) -> LiteralRange<'static> {
        LiteralRange {
            start: self.start.map(|b| Box::new((*b).into_static())),
            end: self.end.map(|b| Box::new((*b).into_static())),
        }
    }
}

impl<'a> fmt::Display for LiteralRange<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bound(&self.start, '(', '[', f)?;
        f.write_str(",")?;
        fmt_bound(&self.end, ')', ']', f)
    }
}
