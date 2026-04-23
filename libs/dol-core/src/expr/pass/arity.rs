//! [`ArityPass`] — validate function argument counts against their [`FuncDef`].
//!
//! This replaces the inline arity check inside `compile_expr`, centralising
//! validation before any rendering takes place.

use crate::expr::arena::ExprArena;
use crate::expr::func_def::Arity;
use crate::expr::node::ExprNode;
use super::PassError;

/// Validates arity of every [`ExprNode::Func`] node in the arena.
pub struct ArityPass;

impl ArityPass {
    /// Iterate all nodes and collect arity violations.
    pub fn check(&self, arena: &ExprArena) -> Vec<PassError> {
        let mut errors = Vec::new();
        for idx in 0..arena.len() {
            let node = arena.get(crate::expr::node::NodeId(idx as u32));
            if let ExprNode::Func(func_node) = node {
                let def = arena.func(func_node.id);
                let actual = func_node.args.len();
                if let Err(e) = def.validate_arity(actual) {
                    errors.push(PassError::BadArity {
                        func_name: def.name().to_owned(),
                        expected:  arity_description(e.expected),
                        actual,
                    });
                }
            }
        }
        errors
    }
}

fn arity_description(a: Arity) -> String {
    match a {
        Arity::Exact(n)      => format!("exactly {n}"),
        Arity::AtLeast(n)    => format!("at least {n}"),
        Arity::Range(lo, hi) => format!("{lo}..={hi}"),
        Arity::Any           => "any".to_owned(),
    }
}
