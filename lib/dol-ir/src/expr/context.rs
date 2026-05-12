//! Evaluation scope — `Context<'a>`.
//!
//! Per `dol-rewrite-plan-v2.md` §8.3. A [`Context`] describes the
//! dynamic scope inside which an expression is evaluated when wrapped
//! in [`Expr::Scoped`]:
//!
//! - **`partition_by`** — keys that partition the input into independent
//!   groups (SQL `PARTITION BY`, dataframe `groupby`, IoT stream key,
//!   pipeline shard).
//! - **`order_by`** — ordering applied within each partition before the
//!   frame and the scoped expression are evaluated.
//! - **`frame`** — optional bounded window over the ordered partition
//!   (rows / range / groups, with [`Boundary`](crate::expr::frame::Boundary)
//!   ends). When `None`, the scope is the entire partition.
//!
//! `Context` is the universal "evaluate inside a scope" descriptor —
//! the same shape covers SQL `OVER (…)`, dataframe groupby+sort, IoT
//! stream slices, time-series segments, and pipeline windows. There is
//! no separate `WindowNode` side pool.
//!
//! ## Status
//!
//! M3c-γ ships the data shape only. The fluent `ContextBuilder` from
//! the plan arrives in M3c-δ together with the lowering pipeline.

extern crate alloc;

use alloc::vec::Vec;

use crate::expr::frame::Frame;
use crate::expr::order::OrderByExpr;
use crate::expr::tree::Expr;

/// The dynamic scope wrapping a [`Scoped`](crate::expr::tree::Expr::Scoped)
/// expression.
///
/// All three fields default to "absent" (empty / `None`). An empty
/// `Context` is valid and means "evaluate against the whole input,
/// in input order, with no windowing" — most backends will treat this
/// as a degenerate scope and elide it during emission.
///
/// ## Validity
///
/// `Context` does not enforce relationships among its fields at
/// construction time:
///
/// - A non-`None` `frame` without an `order_by` is permitted but its
///   meaning is backend-defined; lowering may reject or normalise it.
/// - Empty `partition_by` means "single partition over all input".
///
/// All such checks are the lowering layer's responsibility, where
/// they can produce a proper diagnostic with span context.
#[derive(Debug, Clone, PartialEq)]
pub struct Context<'a> {
    /// Partitioning keys. Empty means a single partition over the
    /// whole input.
    pub partition_by: Vec<Expr<'a>>,
    /// Ordering applied within each partition. Empty means "input
    /// order" (backend-defined for unordered inputs).
    pub order_by: Vec<OrderByExpr<'a>>,
    /// Optional bounded frame within the ordered partition. `None`
    /// means the scope is the entire partition.
    pub frame: Option<Frame>,
}

impl<'a> Context<'a> {
    /// An empty scope: no partitioning, no ordering, no frame.
    ///
    /// Equivalent to evaluating the wrapped expression against the
    /// whole input. Backends typically elide this during emission.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            partition_by: Vec::new(),
            order_by: Vec::new(),
            frame: None,
        }
    }

    /// Returns `true` iff the scope carries no partitioning, no
    /// ordering, and no frame.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.partition_by.is_empty() && self.order_by.is_empty() && self.frame.is_none()
    }
}

impl Default for Context<'_> {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::frame::{Boundary, Frame};
    use crate::expr::order::{NullsOrder, SortDirection};
    use dol_core::path::Path;

    #[test]
    fn empty_context_is_empty() {
        let c = Context::empty();
        assert!(c.is_empty());
        assert!(c.partition_by.is_empty());
        assert!(c.order_by.is_empty());
        assert!(c.frame.is_none());
    }

    #[test]
    fn default_matches_empty() {
        assert_eq!(Context::default(), Context::empty());
    }

    #[test]
    fn populated_context_is_not_empty() {
        let c = Context {
            partition_by: alloc::vec![Expr::Ref(Path::new("user"))],
            order_by: alloc::vec![],
            frame: None,
        };
        assert!(!c.is_empty());
    }

    #[test]
    fn context_round_trips_through_clone_and_eq() {
        let c = Context {
            partition_by: alloc::vec![Expr::Ref(Path::new("user"))],
            order_by: alloc::vec![OrderByExpr {
                expr: Expr::Ref(Path::new("ts")),
                dir: SortDirection::Desc,
                nulls: NullsOrder::First,
            }],
            frame: Some(Frame::rows(
                Boundary::unbounded_preceding(),
                Boundary::Current,
            )),
        };
        let c2 = c.clone();
        assert_eq!(c, c2);
        assert!(!c.is_empty());
    }
}
