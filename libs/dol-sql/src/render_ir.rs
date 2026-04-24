//! Arena-aware SQL renderer for `dol-ir::Statement`.
//!
//! This module renders the new IR layer (`dol-ir`) into SQL strings.
//! It is the successor to the `render` module which renders the old
//! `dol-core::op` IR with `Expr<'a>` lifetime-parameterised types.
//!
//! # DML rendering context
//!
//! DML variants (`Query`, `Insert`, `Update`, `Delete`, `Upsert`) store
//! expression references as [`NodeId`]s into an [`ExprArena`].  Call
//! [`render_ir_statement`] and pass the arena + interner that were used when
//! building the statement.
//!
//! DDL / control / storage / transaction variants are self-contained and do not
//! reference the arena.

use dol_expr::arena::{CaseNode, ExprArena, FieldStep};
use dol_expr::expr::{
    BinOp, ConflictClause, DeleteNode, ExprNode, InsertNode, JoinType, LockHint, Order, QueryNode,
    UnaryOp, UpdateNode, UpsertNode,
};
use dol_expr::ids::{NodeId, NULL_NODE};
use dol_expr::interner::Interner;
use dol_expr::types::Literal;
use dol_ir::control::{DefinePolicy, Grant, PolicyAction, Privilege, Revoke};
use dol_ir::definition::{
    AlterAction, AlterEntity, DefineEntity, DefineIndex, DefineType, DropEntity, DropIndex,
    DropType, FieldDef, IndexMethod, OwnedEntityConstraint, OwnedForeignKeyRef,
};
use dol_ir::entity_ref::EntityRef;
use dol_ir::statement::Statement;
use dol_ir::storage::{GetObject, ListObjects, MoveFile, PutObject, ReadFile, WriteFile};
use dol_ir::transaction::Transaction;
use dol_core::constraint::{FkAction, GeneratedKind};
use dol_types::DataType;

use crate::dialect::{Dialect, PaginationStyle, ParamCounter, ReturningStyle};
use crate::SqlOutput;

// ---------------------------------------------------------------------------
// Top-level statement dispatcher
// ---------------------------------------------------------------------------

/// Context required for rendering DML statements that reference the arena.
pub struct RenderCtx<'a> {
    pub arena:    &'a ExprArena,
    pub interner: &'a Interner,
}

/// Render a `dol-ir` [`Statement`] to SQL.
///
/// `ctx` is only needed for DML statements (Query, Insert, Update, Delete,
/// Upsert).  Pass `None` when the statement is known to be DDL / control /
/// storage / transaction (those do not reference the arena).
pub fn render_ir_statement(
    stmt: &Statement,
    ctx: Option<&RenderCtx<'_>>,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    match stmt {
        // ── DML (arena-based) ─────────────────────────────────────────────
        Statement::Query(q) => {
            let ctx = ctx.ok_or_else(|| BackendError::Unsupported(
                "RenderCtx (arena + interner) required for Query".into(),
            ))?;
            render_query(q, ctx, dialect)
        }
        Statement::Insert(q) => {
            let ctx = ctx.ok_or_else(|| BackendError::Unsupported(
                "RenderCtx (arena + interner) required for Insert".into(),
            ))?;
            render_insert(q, ctx, dialect)
        }
        Statement::Update(q) => {
            let ctx = ctx.ok_or_else(|| BackendError::Unsupported(
                "RenderCtx (arena + interner) required for Update".into(),
            ))?;
            render_update(q, ctx, dialect)
        }
        Statement::Delete(q) => {
            let ctx = ctx.ok_or_else(|| BackendError::Unsupported(
                "RenderCtx (arena + interner) required for Delete".into(),
            ))?;
            render_delete(q, ctx, dialect)
        }
        Statement::Upsert(q) => {
            let ctx = ctx.ok_or_else(|| BackendError::Unsupported(
                "RenderCtx (arena + interner) required for Upsert".into(),
            ))?;
            render_upsert(q, ctx, dialect)
        }

        // ── DDL ───────────────────────────────────────────────────────────
        Statement::DefineEntity(ir) => render_define_entity(ir, dialect),
        Statement::AlterEntity(ir)  => render_alter_entity(ir, dialect),
        Statement::DropEntity(ir)   => render_drop_entity(ir),
        Statement::DefineIndex(ir)  => render_define_index(ir),
        Statement::DropIndex(ir)    => render_drop_index(ir),
        Statement::DefineType(ir)   => render_define_type(ir, dialect),
        Statement::DropType(ir)     => render_drop_type(ir, dialect),

        // ── Access control ────────────────────────────────────────────────
        Statement::Grant(ir)  => render_grant(ir),
        Statement::Revoke(ir) => render_revoke(ir),
        Statement::DefinePolicy(ir) => {
            let ctx = ctx.ok_or_else(|| BackendError::Unsupported(
                "RenderCtx required for DefinePolicy with expression filters".into(),
            ))?;
            render_define_policy(ir, ctx, dialect)
        }

        // ── Transaction ───────────────────────────────────────────────────
        Statement::Transaction(ir) => render_transaction(ir, ctx, dialect),

        // ── Storage ───────────────────────────────────────────────────────
        Statement::PutObject(_)
        | Statement::GetObject(_)
        | Statement::ListObjects(_)
        | Statement::ReadFile(_)
        | Statement::WriteFile(_)
        | Statement::MoveFile(_) => Err(BackendError::Unsupported(
            "storage operations have no SQL equivalent".into(),
        )),

        // ── Raw ───────────────────────────────────────────────────────────
        Statement::Raw(sql) => Ok(SqlOutput { sql: sql.clone(), param_count: 0 }),
    }
}

// ---------------------------------------------------------------------------
// Error type (re-exported from dol-ir so callers don't need both deps)
// ---------------------------------------------------------------------------

pub use dol_ir::backend::BackendError;

// ---------------------------------------------------------------------------
// Arena expression rendering
// ---------------------------------------------------------------------------

struct ArenaRenderer<'a, 'c> {
    arena:    &'a ExprArena,
    interner: &'a Interner,
    dialect:  &'a Dialect,
    counter:  &'c mut ParamCounter,
}

impl<'a, 'c> ArenaRenderer<'a, 'c> {
    fn new(
        arena: &'a ExprArena,
        interner: &'a Interner,
        dialect: &'a Dialect,
        counter: &'c mut ParamCounter,
    ) -> Self {
        Self { arena, interner, dialect, counter }
    }

    fn str(&self, id: dol_expr::ids::StrId) -> &str {
        self.interner.get(id)
    }

    fn render_node(&mut self, id: NodeId) -> Result<String, BackendError> {
        if id == NULL_NODE {
            return Ok(String::new());
        }
        let node = self.arena.get(id).clone();
        self.render_expr_node(&node)
    }

    fn render_expr_node(&mut self, node: &ExprNode) -> Result<String, BackendError> {
        match node {
            ExprNode::Namespace(sid) => Ok(self.str(*sid).to_string()),
            ExprNode::Field(fid) => {
                let f = self.arena.get_field(*fid).clone();
                let col = self.str(f.column).to_string();
                let mut s = if let Some(ns_id) = f.namespace {
                    format!("{}.{}", self.str(ns_id), col)
                } else {
                    col
                };
                // JSON traversal steps
                for step in &f.steps {
                    match step {
                        FieldStep::Key(k_id) => {
                            let k = self.str(*k_id).to_string();
                            s = match &self.dialect.json_access {
                                crate::dialect::JsonAccessStyle::ArrowOperator => {
                                    format!("{}->>'{}'", s, k)
                                }
                                crate::dialect::JsonAccessStyle::JsonExtractFunction => {
                                    format!("json_extract({}, '$.{}')", s, k)
                                }
                                crate::dialect::JsonAccessStyle::JsonValueFunction => {
                                    format!("JSON_VALUE({}, '$.{}')", s, k)
                                }
                                crate::dialect::JsonAccessStyle::Unsupported => {
                                    format!("{}.{}", s, k)
                                }
                            };
                        }
                        FieldStep::Index(idx) => {
                            s = match &self.dialect.json_access {
                                crate::dialect::JsonAccessStyle::ArrowOperator => {
                                    format!("{}->>'{}'", s, idx)
                                }
                                _ => format!("{}[{}]", s, idx),
                            };
                        }
                    }
                }
                Ok(s)
            }
            ExprNode::Param => Ok(self.counter.next()),
            ExprNode::Lit(lit_id) => {
                let lit = self.arena.get_lit(*lit_id).clone();
                Ok(render_literal(&lit, self.dialect))
            }
            ExprNode::ObjectLit(obj_id) => {
                let obj = self.arena.get_obj_lit(*obj_id).clone();
                let pairs: Vec<_> = obj.0.iter()
                    .map(|(k_id, v_id)| {
                        let k = self.str(*k_id).to_string();
                        let v = self.render_node(*v_id)?;
                        Ok(format!("'{}': {}", k, v))
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                Ok(format!("{{{}}}", pairs.join(", ")))
            }
            ExprNode::ArrayLit(items) => {
                let items_clone = items.clone();
                let parts: Vec<_> = items_clone.iter()
                    .map(|id| self.render_node(*id))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("ARRAY[{}]", parts.join(", ")))
            }
            ExprNode::BinOp { op, lhs, rhs } => {
                let lhs_s = self.render_node(*lhs)?;
                let rhs_s = self.render_node(*rhs)?;
                let op_s = binop_token(op);
                Ok(format!("({} {} {})", lhs_s, op_s, rhs_s))
            }
            ExprNode::UnaryOp { op, operand } => {
                let inner = self.render_node(*operand)?;
                Ok(match op {
                    UnaryOp::Neg      => format!("(-{})", inner),
                    UnaryOp::Not      => format!("(NOT {})", inner),
                    UnaryOp::IsNull   => format!("({} IS NULL)", inner),
                    UnaryOp::IsNotNull => format!("({} IS NOT NULL)", inner),
                    UnaryOp::IsTrue   => format!("({} IS TRUE)", inner),
                    UnaryOp::IsFalse  => format!("({} IS FALSE)", inner),
                })
            }
            ExprNode::Func(func_id) => {
                let func = self.arena.get_func(*func_id).clone();
                let name = self.str(func.name).to_string();
                let args: Vec<_> = func.args.iter()
                    .map(|id| self.render_node(*id))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{}({})", name, args.join(", ")))
            }
            ExprNode::Agg { func, expr, distinct } => {
                let name = self.str(*func).to_string();
                let inner = self.render_node(*expr)?;
                let distinct_s = if *distinct { "DISTINCT " } else { "" };
                Ok(format!("{}({}{})", name, distinct_s, inner))
            }
            ExprNode::Window(win_id) => {
                let win = self.arena.get_window(*win_id).clone();
                let name = self.str(win.func).to_string();
                let partition: Vec<_> = win.partition.iter()
                    .map(|id| self.render_node(*id))
                    .collect::<Result<Vec<_>, _>>()?;
                let order: Vec<_> = win.order.iter()
                    .map(|(id, ord)| {
                        let e = self.render_node(*id)?;
                        let d = match ord { Order::Asc => "ASC", Order::Desc => "DESC" };
                        Ok(format!("{} {}", e, d))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let mut over = String::from("OVER (");
                if !partition.is_empty() {
                    over.push_str(&format!("PARTITION BY {}", partition.join(", ")));
                }
                if !order.is_empty() {
                    if !partition.is_empty() { over.push(' '); }
                    over.push_str(&format!("ORDER BY {}", order.join(", ")));
                }
                over.push(')');
                Ok(format!("{}() {}", name, over))
            }
            ExprNode::Cast { expr, to } => {
                let inner = self.render_node(*expr)?;
                let type_name = self.str(*to).to_string();
                Ok(format!("CAST({} AS {})", inner, type_name))
            }
            ExprNode::Case(case_id) => {
                let case = self.arena.get_case(*case_id).clone();
                let CaseNode { branches, else_ } = case;
                let mut s = String::from("CASE");
                for (when_id, then_id) in &branches {
                    let w = self.render_node(*when_id)?;
                    let t = self.render_node(*then_id)?;
                    s.push_str(&format!(" WHEN {} THEN {}", w, t));
                }
                if else_ != NULL_NODE {
                    let e = self.render_node(else_)?;
                    s.push_str(&format!(" ELSE {}", e));
                }
                s.push_str(" END");
                Ok(s)
            }
            ExprNode::Alias { expr, name } => {
                let inner = self.render_node(*expr)?;
                let alias = self.str(*name).to_string();
                Ok(format!("{} AS {}", inner, alias))
            }
            ExprNode::InList(in_id) => {
                let inlist = self.arena.get_in_list(*in_id).clone();
                let expr_s = self.render_node(inlist.expr)?;
                let items: Vec<_> = inlist.list.iter()
                    .map(|id| self.render_node(*id))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("({} IN ({}))", expr_s, items.join(", ")))
            }
            ExprNode::InSub { expr, sub } => {
                let e = self.render_node(*expr)?;
                let s = self.render_node(*sub)?;
                Ok(format!("({} IN ({}))", e, s))
            }
            ExprNode::Exists { sub } => {
                let s = self.render_node(*sub)?;
                Ok(format!("EXISTS ({})", s))
            }
            ExprNode::IsNull { expr } => {
                let e = self.render_node(*expr)?;
                Ok(format!("({} IS NULL)", e))
            }
            ExprNode::Between { expr, lo, hi } => {
                let e = self.render_node(*expr)?;
                let l = self.render_node(*lo)?;
                let h = self.render_node(*hi)?;
                Ok(format!("({} BETWEEN {} AND {})", e, l, h))
            }
            ExprNode::Query(qid) => {
                let q = self.arena.get_query(*qid).clone();
                // Render a sub-query (same counter for parameter continuity)
                let sub_out = render_query_with_counter(&q, self.arena, self.interner, self.dialect, self.counter)?;
                Ok(sub_out.sql)
            }
            ExprNode::Insert(iid) => {
                let ins = self.arena.get_insert(*iid).clone();
                let out = render_insert_with_counter(&ins, self.arena, self.interner, self.dialect, self.counter)?;
                Ok(out.sql)
            }
            ExprNode::Update(uid) => {
                let upd = self.arena.get_update(*uid).clone();
                let out = render_update_with_counter(&upd, self.arena, self.interner, self.dialect, self.counter)?;
                Ok(out.sql)
            }
            ExprNode::Delete(did) => {
                let del = self.arena.get_delete(*did).clone();
                let out = render_delete_with_counter(&del, self.arena, self.interner, self.dialect, self.counter)?;
                Ok(out.sql)
            }
            ExprNode::Upsert(up_id) => {
                let ups = self.arena.get_upsert(*up_id).clone();
                let out = render_upsert_with_counter(&ups, self.arena, self.interner, self.dialect, self.counter)?;
                Ok(out.sql)
            }
        }
    }
}

fn binop_token(op: &BinOp) -> &'static str {
    match op {
        BinOp::Eq       => "=",
        BinOp::Ne       => "!=",
        BinOp::Lt       => "<",
        BinOp::Le       => "<=",
        BinOp::Gt       => ">",
        BinOp::Ge       => ">=",
        BinOp::And      => "AND",
        BinOp::Or       => "OR",
        BinOp::Add      => "+",
        BinOp::Sub      => "-",
        BinOp::Mul      => "*",
        BinOp::Div      => "/",
        BinOp::Rem      => "%",
        BinOp::Like     => "LIKE",
        BinOp::ILike    => "ILIKE",
        BinOp::Similar  => "SIMILAR TO",
        BinOp::BitAnd   => "&",
        BinOp::BitOr    => "|",
        BinOp::BitXor   => "#",
        BinOp::Shl      => "<<",
        BinOp::Shr      => ">>",
        BinOp::Concat   => "||",
        BinOp::Arrow    => "->",
        BinOp::LongArrow => "->>",
    }
}

fn render_literal(lit: &Literal<'_>, dialect: &Dialect) -> String {
    use dol_expr::types::Literal as L;
    match lit {
        L::Null         => "NULL".to_string(),
        L::Bool(true)   => dialect.bool_true.clone(),
        L::Bool(false)  => dialect.bool_false.clone(),
        L::String(s)    => format!("'{}'", escape_sql_string(s, dialect)),
        L::Json(s)      => format!("'{}'", escape_sql_string(s, dialect)),
        L::Xml(s)       => format!("'{}'", escape_sql_string(s, dialect)),
        L::Enum(s)      => format!("'{}'", escape_sql_string(s, dialect)),
        L::Bytes(b)     => {
            let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
            format!("'\\x{}'", hex)
        }
        L::Uuid(_)      => format!("'{}'", lit),
        L::BitString(bs) => format!("B'{}'", bs),
        L::Int8(n)      => n.to_string(),
        L::Int16(n)     => n.to_string(),
        L::Int32(n)     => n.to_string(),
        L::Int64(n)     => n.to_string(),
        L::Int128(n)    => n.to_string(),
        L::UInt8(n)     => n.to_string(),
        L::UInt16(n)    => n.to_string(),
        L::UInt32(n)    => n.to_string(),
        L::UInt64(n)    => n.to_string(),
        L::UInt128(n)   => n.to_string(),
        L::Float32(f)   => f.to_string(),
        L::Float64(f)   => f.to_string(),
        L::Decimal(d)   => d.to_string(),
        L::Inet(a)      => format!("'{}'", a),
        L::MacAddr(m)   => format!("'{}'", m),
        L::MacAddr8(m)  => format!("'{}'", m),
        L::Date(d)      => format!("'{}'", d),
        L::Time(t)      => format!("'{}'", t),
        L::DateTime(dt) => format!("'{}'", dt),
        L::TimestampTz(ts) => format!("'{}'", ts),
        L::Interval(iv) => format!("'{}'", iv),
        L::Point(p)     => format!("'{}'", p),
        L::Line(l)      => format!("'{}'", l),
        L::Segment(s)   => format!("'{}'", s),
        L::Rect(r)      => format!("'{}'", r),
        L::Circle(c)    => format!("'{}'", c),
        L::Path(p)      => format!("'{}'", p),
        L::Polygon(p)   => format!("'{}'", p),
        L::Array(_) | L::Set(_) | L::Tuple(_) | L::Map(_) | L::Struct(_) | L::Range(_) => {
            format!("'{}'", lit)
        }
        L::Extension(..) => format!("'{}'", lit),
    }
}

fn escape_sql_string(s: &str, _dialect: &Dialect) -> String {
    s.replace('\'', "''")
}

// ---------------------------------------------------------------------------
// DML renderers
// ---------------------------------------------------------------------------

fn render_query(q: &QueryNode, ctx: &RenderCtx<'_>, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = dialect.param_counter();
    render_query_with_counter(q, ctx.arena, ctx.interner, dialect, &mut counter)
}

fn render_query_with_counter(
    q:        &QueryNode,
    arena:    &ExprArena,
    interner: &Interner,
    dialect:  &Dialect,
    counter:  &mut ParamCounter,
) -> Result<SqlOutput, BackendError> {
    let mut r = ArenaRenderer::new(arena, interner, dialect, counter);

    // SELECT columns
    let mut sql = String::from("SELECT ");
    if q.columns.is_empty() {
        sql.push('*');
    } else {
        let cols: Vec<_> = q.columns.iter()
            .map(|id| r.render_node(*id))
            .collect::<Result<Vec<_>, _>>()?;
        sql.push_str(&cols.join(", "));
    }

    // FROM
    let table = r.str(q.from).to_string();
    sql.push_str(&format!(" FROM {}", table));
    if let Some(alias_id) = q.alias {
        sql.push_str(&format!(" AS {}", r.str(alias_id)));
    }

    // JOINs
    for join in &q.joins {
        let join_kw = match join.join_type {
            JoinType::Inner => "JOIN",
            JoinType::Left  => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full  => "FULL JOIN",
            JoinType::Cross => "CROSS JOIN",
        };
        let src = r.str(join.source).to_string();
        sql.push_str(&format!(" {} {}", join_kw, src));
        if let Some(a) = join.alias {
            sql.push_str(&format!(" AS {}", r.str(a)));
        }
        if join.on != NULL_NODE {
            let on_s = r.render_node(join.on)?;
            sql.push_str(&format!(" ON {}", on_s));
        }
    }

    // WHERE
    if q.filter != NULL_NODE {
        let f = r.render_node(q.filter)?;
        sql.push_str(&format!(" WHERE {}", f));
    }

    // GROUP BY
    if !q.group_by.is_empty() {
        let gb: Vec<_> = q.group_by.iter()
            .map(|id| r.render_node(*id))
            .collect::<Result<Vec<_>, _>>()?;
        sql.push_str(&format!(" GROUP BY {}", gb.join(", ")));
    }

    // HAVING
    if q.having != NULL_NODE {
        let h = r.render_node(q.having)?;
        sql.push_str(&format!(" HAVING {}", h));
    }

    // ORDER BY
    if !q.order_by.is_empty() {
        let ob: Vec<_> = q.order_by.iter()
            .map(|(id, ord)| {
                let e = r.render_node(*id)?;
                let d = match ord { Order::Asc => "ASC", Order::Desc => "DESC" };
                Ok(format!("{} {}", e, d))
            })
            .collect::<Result<Vec<_>, _>>()?;
        sql.push_str(&format!(" ORDER BY {}", ob.join(", ")));
    }

    // LIMIT / OFFSET
    match &dialect.pagination {
        PaginationStyle::LimitOffset => {
            if let Some(offset) = q.offset { sql.push_str(&format!(" OFFSET {}", offset)); }
            if let Some(limit)  = q.limit  { sql.push_str(&format!(" LIMIT {}", limit)); }
        }
        PaginationStyle::OffsetFetch => {
            if let Some(offset) = q.offset {
                sql.push_str(&format!(" OFFSET {} ROWS", offset));
            } else if q.limit.is_some() {
                sql.push_str(" OFFSET 0 ROWS");
            }
            if let Some(limit) = q.limit {
                sql.push_str(&format!(" FETCH NEXT {} ROWS ONLY", limit));
            }
        }
        PaginationStyle::Rownum => {}
    }

    // LOCK
    if let Some(lock) = &q.lock {
        let lock_s = match lock {
            LockHint::ForUpdate    => " FOR UPDATE",
            LockHint::ForShare     => " FOR SHARE",
            LockHint::SkipLocked   => " FOR UPDATE SKIP LOCKED",
            LockHint::NoWait       => " FOR UPDATE NOWAIT",
        };
        sql.push_str(lock_s);
    }

    let param_count = r.counter.count();
    Ok(SqlOutput { sql, param_count })
}

fn render_insert(ins: &InsertNode, ctx: &RenderCtx<'_>, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = dialect.param_counter();
    render_insert_with_counter(ins, ctx.arena, ctx.interner, dialect, &mut counter)
}

fn render_insert_with_counter(
    ins:      &InsertNode,
    arena:    &ExprArena,
    interner: &Interner,
    dialect:  &Dialect,
    counter:  &mut ParamCounter,
) -> Result<SqlOutput, BackendError> {
    let mut r = ArenaRenderer::new(arena, interner, dialect, counter);

    let table = r.str(ins.target).to_string();
    let cols: Vec<_> = ins.columns.iter().map(|id| r.str(*id).to_string()).collect();
    let vals: Vec<_> = ins.values.iter()
        .map(|id| r.render_node(*id))
        .collect::<Result<Vec<_>, _>>()?;

    let mut sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table,
        cols.join(", "),
        vals.join(", ")
    );

    // ON CONFLICT
    if let Some(ref conflict) = ins.conflict {
        match conflict {
            ConflictClause::DoNothing => sql.push_str(" ON CONFLICT DO NOTHING"),
            ConflictClause::DoUpdate { assignments } => {
                sql.push_str(" ON CONFLICT DO UPDATE SET ");
                let sets: Vec<_> = assignments.iter()
                    .map(|(col_id, val_id)| {
                        let col = r.str(*col_id).to_string();
                        let val = r.render_node(*val_id)?;
                        Ok(format!("{} = {}", col, val))
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                sql.push_str(&sets.join(", "));
            }
        }
    }

    // RETURNING
    let returning = render_returning_nodes(&ins.returning, &mut r, dialect)?;
    sql.push_str(&returning);

    let param_count = r.counter.count();
    Ok(SqlOutput { sql, param_count })
}

fn render_update(upd: &UpdateNode, ctx: &RenderCtx<'_>, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = dialect.param_counter();
    render_update_with_counter(upd, ctx.arena, ctx.interner, dialect, &mut counter)
}

fn render_update_with_counter(
    upd:      &UpdateNode,
    arena:    &ExprArena,
    interner: &Interner,
    dialect:  &Dialect,
    counter:  &mut ParamCounter,
) -> Result<SqlOutput, BackendError> {
    let mut r = ArenaRenderer::new(arena, interner, dialect, counter);
    let table = r.str(upd.target).to_string();

    let sets: Vec<_> = upd.columns.iter().zip(upd.values.iter())
        .map(|(col_id, val_id)| {
            let col = r.str(*col_id).to_string();
            let val = r.render_node(*val_id)?;
            Ok(format!("{} = {}", col, val))
        })
        .collect::<Result<Vec<_>, BackendError>>()?;

    let mut sql = format!("UPDATE {} SET {}", table, sets.join(", "));

    if upd.filter != NULL_NODE {
        let f = r.render_node(upd.filter)?;
        sql.push_str(&format!(" WHERE {}", f));
    }

    let returning = render_returning_nodes(&upd.returning, &mut r, dialect)?;
    sql.push_str(&returning);

    let param_count = r.counter.count();
    Ok(SqlOutput { sql, param_count })
}

fn render_delete(del: &DeleteNode, ctx: &RenderCtx<'_>, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = dialect.param_counter();
    render_delete_with_counter(del, ctx.arena, ctx.interner, dialect, &mut counter)
}

fn render_delete_with_counter(
    del:      &DeleteNode,
    arena:    &ExprArena,
    interner: &Interner,
    dialect:  &Dialect,
    counter:  &mut ParamCounter,
) -> Result<SqlOutput, BackendError> {
    let mut r = ArenaRenderer::new(arena, interner, dialect, counter);
    let table = r.str(del.target).to_string();
    let mut sql = format!("DELETE FROM {}", table);

    if del.filter != NULL_NODE {
        let f = r.render_node(del.filter)?;
        sql.push_str(&format!(" WHERE {}", f));
    }

    let returning = render_returning_nodes(&del.returning, &mut r, dialect)?;
    sql.push_str(&returning);

    let param_count = r.counter.count();
    Ok(SqlOutput { sql, param_count })
}

fn render_upsert(ups: &UpsertNode, ctx: &RenderCtx<'_>, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut counter = dialect.param_counter();
    render_upsert_with_counter(ups, ctx.arena, ctx.interner, dialect, &mut counter)
}

fn render_upsert_with_counter(
    ups:      &UpsertNode,
    arena:    &ExprArena,
    interner: &Interner,
    dialect:  &Dialect,
    counter:  &mut ParamCounter,
) -> Result<SqlOutput, BackendError> {
    // Upsert is basically an Insert with conflict handling
    let mut r = ArenaRenderer::new(arena, interner, dialect, counter);
    let table = r.str(ups.target).to_string();
    let cols: Vec<_> = ups.columns.iter().map(|id| r.str(*id).to_string()).collect();
    let vals: Vec<_> = ups.values.iter()
        .map(|id| r.render_node(*id))
        .collect::<Result<Vec<_>, _>>()?;

    let mut sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table,
        cols.join(", "),
        vals.join(", ")
    );

    if let Some(ref conflict) = ups.conflict {
        match conflict {
            ConflictClause::DoNothing => sql.push_str(" ON CONFLICT DO NOTHING"),
            ConflictClause::DoUpdate { assignments } => {
                sql.push_str(" ON CONFLICT DO UPDATE SET ");
                let sets: Vec<_> = assignments.iter()
                    .map(|(col_id, val_id)| {
                        let col = r.str(*col_id).to_string();
                        let val = r.render_node(*val_id)?;
                        Ok(format!("{} = {}", col, val))
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                sql.push_str(&sets.join(", "));
            }
        }
    }

    let returning = render_returning_nodes(&ups.returning, &mut r, dialect)?;
    sql.push_str(&returning);

    let param_count = r.counter.count();
    Ok(SqlOutput { sql, param_count })
}

fn render_returning_nodes(
    returning: &[NodeId],
    r:         &mut ArenaRenderer<'_, '_>,
    dialect:   &Dialect,
) -> Result<String, BackendError> {
    if returning.is_empty() {
        return Ok(String::new());
    }
    match &dialect.returning_style {
        ReturningStyle::Returning | ReturningStyle::ReturningInto => {
            let cols: Vec<_> = returning.iter()
                .map(|id| r.render_node(*id))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!(" RETURNING {}", cols.join(", ")))
        }
        ReturningStyle::OutputInserted => {
            let cols: Vec<_> = returning.iter()
                .map(|id| r.render_node(*id).map(|c| format!("INSERTED.{}", c)))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!(" OUTPUT {}", cols.join(", ")))
        }
        ReturningStyle::Unsupported => Ok(String::new()),
    }
}

// ---------------------------------------------------------------------------
// DDL renderers
// ---------------------------------------------------------------------------

pub fn render_define_entity(ir: &DefineEntity, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let mut sql = String::from("CREATE TABLE ");
    if ir.if_not_exists {
        sql.push_str("IF NOT EXISTS ");
    }
    let name = qualified_name(&ir.name, ir.namespace.as_deref());
    sql.push_str(&name);
    sql.push_str(" (\n");

    let mut parts = Vec::new();
    for fd in &ir.fields {
        parts.push(format!("  {}", render_field_def_ir(fd, dialect)));
    }

    // Derive a table-level PRIMARY KEY constraint from fields if none is present.
    let has_explicit_pk = ir.constraints.iter().any(|c| {
        matches!(c, OwnedEntityConstraint::PrimaryKey(_))
    });
    if !has_explicit_pk {
        let pk_cols: Vec<String> = ir.fields.iter()
            .filter(|f| f.primary_key)
            .map(|f| f.name.clone())
            .collect();
        if !pk_cols.is_empty() {
            parts.push(format!("  PRIMARY KEY ({})", pk_cols.join(", ")));
        }
    }

    for c in &ir.constraints {
        parts.push(format!("  {}", render_entity_constraint(c)));
    }
    sql.push_str(&parts.join(",\n"));
    sql.push_str("\n)");
    Ok(SqlOutput { sql, param_count: 0 })
}

pub fn render_alter_entity(ir: &AlterEntity, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    let table_name = entity_ref_to_sql(&ir.target, dialect);
    let mut parts = Vec::new();

    for action in &ir.actions {
        let part = match action {
            AlterAction::AddField(fd) => format!(
                "ALTER TABLE {} ADD COLUMN {}",
                table_name, render_field_def_ir(fd, dialect)
            ),
            AlterAction::DropField(name) => format!(
                "ALTER TABLE {} DROP COLUMN {}",
                table_name, name
            ),
            AlterAction::RenameField { from, to } => format!(
                "ALTER TABLE {} RENAME COLUMN {} TO {}",
                table_name, from, to
            ),
            AlterAction::AlterFieldType { name, new_type } => format!(
                "ALTER TABLE {} ALTER COLUMN {} TYPE {}",
                table_name, name, resolve_type(new_type, dialect)
            ),
            AlterAction::SetFieldDefault { name, expr } => format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {}",
                table_name, name, expr
            ),
            AlterAction::DropFieldDefault(name) => format!(
                "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT",
                table_name, name
            ),
            AlterAction::SetFieldNotNull(name) => format!(
                "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL",
                table_name, name
            ),
            AlterAction::DropFieldNotNull(name) => format!(
                "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL",
                table_name, name
            ),
            AlterAction::AddConstraint(c) => format!(
                "ALTER TABLE {} ADD {}",
                table_name, render_entity_constraint(c)
            ),
            AlterAction::DropConstraint(name) => format!(
                "ALTER TABLE {} DROP CONSTRAINT {}",
                table_name, name
            ),
            AlterAction::RenameEntity(new_name) => format!(
                "ALTER TABLE {} RENAME TO {}",
                table_name, new_name
            ),
        };
        parts.push(part);
    }

    Ok(SqlOutput { sql: parts.join(";\n"), param_count: 0 })
}

pub fn render_drop_entity(ir: &DropEntity) -> Result<SqlOutput, BackendError> {
    let mut sql = String::from("DROP TABLE ");
    if ir.if_exists { sql.push_str("IF EXISTS "); }
    sql.push_str(&qualified_name(&ir.target.name, ir.target.namespace.as_deref()));
    if ir.cascade { sql.push_str(" CASCADE"); }
    Ok(SqlOutput { sql, param_count: 0 })
}

pub fn render_define_index(ir: &DefineIndex) -> Result<SqlOutput, BackendError> {
    let mut sql = String::from("CREATE ");
    if ir.unique { sql.push_str("UNIQUE "); }
    sql.push_str("INDEX ");
    if ir.concurrently { sql.push_str("CONCURRENTLY "); }
    if ir.if_not_exists { sql.push_str("IF NOT EXISTS "); }
    sql.push_str(&ir.name);
    sql.push_str(&format!(" ON {}", ir.target.name));

    if let Some(ref method) = ir.method {
        let m = match method {
            IndexMethod::BTree   => "btree",
            IndexMethod::Hash    => "hash",
            IndexMethod::FullText => "gin",
            IndexMethod::Spatial => "gist",
            IndexMethod::Custom(s) => s.as_str(),
        };
        sql.push_str(&format!(" USING {}", m));
    }

    sql.push_str(&format!(" ({})", ir.columns.join(", ")));
    if let Some(ref wc) = ir.where_clause {
        sql.push_str(&format!(" WHERE {}", wc));
    }
    Ok(SqlOutput { sql, param_count: 0 })
}

pub fn render_drop_index(ir: &DropIndex) -> Result<SqlOutput, BackendError> {
    let mut sql = String::from("DROP INDEX ");
    if ir.concurrently { sql.push_str("CONCURRENTLY "); }
    if ir.if_exists    { sql.push_str("IF EXISTS "); }
    sql.push_str(&ir.name);
    if ir.cascade      { sql.push_str(" CASCADE"); }
    Ok(SqlOutput { sql, param_count: 0 })
}

pub fn render_define_type(ir: &DefineType, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    use crate::dialect::ddl::EnumStyle;
    let qname = qualified_name(&ir.name, ir.namespace.as_deref());
    let sql = match dialect.ddl.enum_style {
        EnumStyle::CreateType => {
            let variants: Vec<_> = ir.variants.iter().map(|v| format!("'{}'", v)).collect();
            format!("CREATE TYPE {} AS ENUM ({})", qname, variants.join(", "))
        }
        EnumStyle::InlineEnum => format!(
            "-- type {} is rendered inline as ENUM({}) in column definitions",
            qname,
            ir.variants.iter().map(|v| format!("'{}'", v)).collect::<Vec<_>>().join(", ")
        ),
        EnumStyle::CheckConstraint => format!(
            "-- type {} is enforced via CHECK (col IN ({})) in column definitions",
            qname,
            ir.variants.iter().map(|v| format!("'{}'", v)).collect::<Vec<_>>().join(", ")
        ),
    };
    Ok(SqlOutput { sql, param_count: 0 })
}

pub fn render_drop_type(ir: &DropType, dialect: &Dialect) -> Result<SqlOutput, BackendError> {
    use crate::dialect::ddl::EnumStyle;
    let sql = match dialect.ddl.enum_style {
        EnumStyle::CreateType => {
            let mut s = String::from("DROP TYPE ");
            if ir.if_exists { s.push_str("IF EXISTS "); }
            s.push_str(&ir.name);
            s
        }
        _ => format!("-- type {} does not exist as a standalone object in this dialect", ir.name),
    };
    Ok(SqlOutput { sql, param_count: 0 })
}

// ---------------------------------------------------------------------------
// Access control renderers
// ---------------------------------------------------------------------------

pub fn render_grant(ir: &Grant) -> Result<SqlOutput, BackendError> {
    let priv_s = render_privilege(&ir.privilege);
    Ok(SqlOutput {
        sql: format!("GRANT {} ON {} TO {}", priv_s, ir.on_target, ir.to_role),
        param_count: 0,
    })
}

pub fn render_revoke(ir: &Revoke) -> Result<SqlOutput, BackendError> {
    let priv_s = render_privilege(&ir.privilege);
    Ok(SqlOutput {
        sql: format!("REVOKE {} ON {} FROM {}", priv_s, ir.on_target, ir.from_role),
        param_count: 0,
    })
}

fn render_privilege(p: &Privilege) -> String {
    match p {
        Privilege::Select  => "SELECT".into(),
        Privilege::Insert  => "INSERT".into(),
        Privilege::Update  => "UPDATE".into(),
        Privilege::Delete  => "DELETE".into(),
        Privilege::All     => "ALL".into(),
        Privilege::Usage   => "USAGE".into(),
        Privilege::Create  => "CREATE".into(),
        Privilege::Connect => "CONNECT".into(),
        Privilege::Custom(s) => s.clone(),
    }
}

pub fn render_define_policy(
    ir:      &DefinePolicy,
    ctx:     &RenderCtx<'_>,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    let mut counter = dialect.param_counter();
    let mut r = ArenaRenderer::new(ctx.arena, ctx.interner, dialect, &mut counter);

    let action_s = match ir.action {
        PolicyAction::Read  => "SELECT",
        PolicyAction::Write => "ALL",
        PolicyAction::All   => "ALL",
    };

    let mut sql = format!("CREATE POLICY {} ON {} FOR {}", ir.name, ir.on_model, action_s);

    if let Some(using_id) = ir.using_expr {
        let e = r.render_node(using_id)?;
        sql.push_str(&format!(" USING ({})", e));
    }
    if let Some(check_id) = ir.check_expr {
        let e = r.render_node(check_id)?;
        sql.push_str(&format!(" WITH CHECK ({})", e));
    }

    Ok(SqlOutput { sql, param_count: r.counter.count() })
}

// ---------------------------------------------------------------------------
// Transaction renderer
// ---------------------------------------------------------------------------

pub fn render_transaction(
    ir:      &Transaction,
    ctx:     Option<&RenderCtx<'_>>,
    dialect: &Dialect,
) -> Result<SqlOutput, BackendError> {
    match ir {
        Transaction::Begin   => Ok(SqlOutput { sql: "BEGIN".into(),    param_count: 0 }),
        Transaction::Commit  => Ok(SqlOutput { sql: "COMMIT".into(),   param_count: 0 }),
        Transaction::Rollback => Ok(SqlOutput { sql: "ROLLBACK".into(), param_count: 0 }),
        Transaction::Savepoint(name)           => Ok(SqlOutput { sql: format!("SAVEPOINT {}", name), param_count: 0 }),
        Transaction::ReleaseSavepoint(name)    => Ok(SqlOutput { sql: format!("RELEASE SAVEPOINT {}", name), param_count: 0 }),
        Transaction::RollbackToSavepoint(name) => Ok(SqlOutput { sql: format!("ROLLBACK TO SAVEPOINT {}", name), param_count: 0 }),
        Transaction::Block(stmts) => {
            let mut parts = vec!["BEGIN".to_string()];
            let mut total_params = 0;
            for stmt in stmts {
                let out = render_ir_statement(stmt, ctx, dialect)?;
                total_params += out.param_count;
                parts.push(out.sql);
            }
            parts.push("COMMIT".to_string());
            Ok(SqlOutput { sql: parts.join(";\n"), param_count: total_params })
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn qualified_name(name: &str, namespace: Option<&str>) -> String {
    match namespace {
        Some(ns) => format!("{}.{}", ns, name),
        None     => name.to_string(),
    }
}

fn entity_ref_to_sql(er: &EntityRef, _dialect: &Dialect) -> String {
    let base = qualified_name(&er.name, er.namespace.as_deref());
    if let Some(ref alias) = er.alias {
        format!("{} AS {}", base, alias)
    } else {
        base
    }
}

fn resolve_type(dt: &DataType, dialect: &Dialect) -> String {
    dialect.resolve_type(dt)
}

pub fn render_field_def_ir(fd: &FieldDef, dialect: &Dialect) -> String {
    let type_str = dialect.resolve_type(&fd.data_type);
    let mut def = format!("{} {}", fd.name, type_str);

    if let Some(ref col) = fd.collation {
        def.push_str(&format!(" COLLATE {}", col));
    }
    if !fd.nullable {
        def.push_str(" NOT NULL");
    }
    if let Some(ref expr) = fd.default_expr {
        def.push_str(&format!(" DEFAULT {}", expr));
    }
    if fd.unique {
        def.push_str(" UNIQUE");
    }
    if let Some(ref fk) = fd.references {
        def.push_str(&render_inline_fk(fk));
    }
    if let Some(ref check_expr) = fd.check {
        def.push_str(&format!(" CHECK ({})", check_expr));
    }
    if let Some((kind, ref expr)) = fd.generated {
        let stored = match kind {
            GeneratedKind::Stored  => "STORED",
            GeneratedKind::Virtual => "VIRTUAL",
        };
        def.push_str(&format!(" GENERATED ALWAYS AS ({}) {}", expr, stored));
    }
    def
}

fn render_inline_fk(fk: &OwnedForeignKeyRef) -> String {
    let mut s = format!(" REFERENCES {}({})", fk.table, fk.column);
    if fk.on_delete != FkAction::NoAction {
        s.push_str(&format!(" ON DELETE {}", fk.on_delete));
    }
    if fk.on_update != FkAction::NoAction {
        s.push_str(&format!(" ON UPDATE {}", fk.on_update));
    }
    s
}

pub fn render_entity_constraint(c: &OwnedEntityConstraint) -> String {
    match c {
        OwnedEntityConstraint::Unique(cols) => format!("UNIQUE ({})", cols.join(", ")),
        OwnedEntityConstraint::ForeignKey { columns, ref_table, ref_columns, on_delete } => {
            let mut s = format!(
                "FOREIGN KEY ({}) REFERENCES {} ({})",
                columns.join(", "), ref_table, ref_columns.join(", ")
            );
            if *on_delete != FkAction::NoAction {
                s.push_str(&format!(" ON DELETE {}", on_delete));
            }
            s
        }
        OwnedEntityConstraint::Check(expr) => format!("CHECK ({})", expr),
        OwnedEntityConstraint::PrimaryKey(cols) => format!("PRIMARY KEY ({})", cols.join(", ")),
    }
}
