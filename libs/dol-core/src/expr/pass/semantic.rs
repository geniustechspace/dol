//! [`SemanticPass`] — context-sensitive expression rules.
//!
//! Tracks *where* an expression lives in a query and rejects constructs that
//! are syntactically valid but semantically illegal (e.g. aggregate functions
//! in a `WHERE` clause).

use std::fmt;

use crate::expr::arena::ExprArena;
use crate::expr::func_def::FuncKind;
use crate::expr::node::ExprNode;
use super::PassError;

/// The position within a query where an expression appears.
///
/// Passed to [`SemanticPass`] so it can apply the right rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprContext {
    /// Projection list (`SELECT …`).
    Select,
    /// Filter predicate (`WHERE …`).
    Where,
    /// Post-aggregation filter (`HAVING …`).
    Having,
    /// Sort key (`ORDER BY …`).
    OrderBy,
    /// Join condition (`ON …`).
    JoinOn,
    /// Group key (`GROUP BY …`).
    GroupBy,
}

impl fmt::Display for ExprContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Select  => "SELECT",
            Self::Where   => "WHERE",
            Self::Having  => "HAVING",
            Self::OrderBy => "ORDER BY",
            Self::JoinOn  => "JOIN ON",
            Self::GroupBy => "GROUP BY",
        })
    }
}

/// Validates context-sensitive semantics for all function nodes in the arena.
///
/// Rules:
/// - Aggregate functions are illegal in `WHERE`, `JoinOn`, and `GroupBy`.
/// - Window functions are illegal in `WHERE`, `JoinOn`, `Having`, and `GroupBy`.
pub struct SemanticPass;

impl SemanticPass {
    pub fn check(&self, arena: &ExprArena, context: ExprContext) -> Vec<PassError> {
        // Fast exit: Select and OrderBy allow both aggregates and windows.
        if matches!(context, ExprContext::Select | ExprContext::OrderBy) {
            return vec![];
        }

        let mut errors = Vec::new();
        for idx in 0..arena.len() {
            let node = arena.get(crate::expr::node::NodeId(idx as u32));
            if let ExprNode::Func(func_node) = node {
                let def = arena.func(func_node.id);
                match def.kind() {
                    FuncKind::Aggregate => {
                        // Aggregates illegal in WHERE, JoinOn, GroupBy.
                        if matches!(context,
                            ExprContext::Where
                            | ExprContext::JoinOn
                            | ExprContext::GroupBy
                        ) {
                            errors.push(PassError::AggregateInWrongContext {
                                func_name: def.name().to_owned(),
                                context,
                            });
                        }
                    }
                    FuncKind::Window => {
                        // Window functions illegal everywhere except Select/OrderBy.
                        errors.push(PassError::WindowInWrongContext {
                            func_name: def.name().to_owned(),
                            context,
                        });
                    }
                    FuncKind::Scalar => {}
                }
            }
        }
        errors
    }
}
