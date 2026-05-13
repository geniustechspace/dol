//! [`ValueRange`]: a bound-pair for [`Value`].

use alloc::boxed::Box;
use core::fmt;
use core::ops::Bound;

use super::Value;

/// A range bound-pair for [`Value`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ValueRange {
    pub start: Bound<Box<Value>>,
    pub end: Bound<Box<Value>>,
}

impl ValueRange {
    pub fn new(start: Bound<Value>, end: Bound<Value>) -> Self {
        Self {
            start: start.map(Box::new),
            end: end.map(Box::new),
        }
    }
}

pub(crate) fn fmt_bound<T: fmt::Display>(
    b: &Bound<Box<T>>,
    open: char,
    closed: char,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match b {
        Bound::Included(v) => write!(f, "{closed}{v}"),
        Bound::Excluded(v) => write!(f, "{open}{v}"),
        Bound::Unbounded => write!(f, "{open}"),
    }
}

impl fmt::Display for ValueRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bound(&self.start, '(', '[', f)?;
        f.write_str(",")?;
        fmt_bound(&self.end, ')', ']', f)
    }
}
