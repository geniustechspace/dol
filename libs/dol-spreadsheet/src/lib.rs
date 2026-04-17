//! # dol-spreadsheet — DOL Spreadsheet Backend
//!
//! Renders DOL IR into spreadsheet operation descriptors.
//!
//! Maps DOL concepts to spreadsheet semantics:
//! - **Entity** → Sheet (tab)
//! - **Field** → Column header
//! - **DataType** → Cell format
//! - **Insert** → Append rows
//! - **Update** → Find & modify rows
//! - **Remove** → Delete rows
//! - **Query** → Filter & select cells
//! - **DefineEntity** → Create sheet with typed columns
//! - **DropEntity** → Delete sheet
//! - **AlterEntity** → Rename sheet
//!
//! Operations not supported by spreadsheets (JOINs, transactions, subqueries,
//! access control, indexes, etc.) return [`BackendError::Unsupported`].

#![deny(unsafe_code)]

use dol_core::expr::{BinOp, Direction, Expr, OrderByExpr, UnaryOp};
use dol_core::ir::{
    Backend, BackendError, RenderedOutput, SortDirection, SpreadsheetColumnDef, SpreadsheetOp,
    SpreadsheetOutput, SpreadsheetSortSpec, Statement,
};

/// Backend that renders DOL IR statements into [`SpreadsheetOutput`] descriptors.
///
/// # Supported operations
///
/// | DOL Statement | Spreadsheet Operation |
/// |---------------|----------------------|
/// | `DefineEntity` | `CreateSheet` |
/// | `DropEntity` | `DropSheet` |
/// | `AlterEntity` (rename only) | `RenameSheet` |
/// | `Insert` | `AppendRows` |
/// | `Update` | `UpdateRows` |
/// | `Remove` | `DeleteRows` |
/// | `Query` | `ReadRows` |
///
/// All other statements return [`BackendError::Unsupported`].
pub struct SpreadsheetBackend;

impl Backend for SpreadsheetBackend {
    fn render(&self, stmt: &Statement<'_>) -> Result<RenderedOutput, BackendError> {
        match stmt {
            // ── Definition ──────────────────────────────────────────────
            Statement::DefineEntity(ir) => {
                let columns = ir
                    .fields
                    .iter()
                    .map(|f| SpreadsheetColumnDef {
                        name: f.name.clone(),
                        data_type: f.data_type.clone(),
                    })
                    .collect();

                let sheet = qualified_name(&ir.namespace, &ir.name);

                Ok(RenderedOutput::Spreadsheet(SpreadsheetOutput {
                    operation: SpreadsheetOp::CreateSheet {
                        columns,
                        if_not_exists: ir.if_not_exists,
                    },
                    sheet,
                    workbook: None,
                }))
            }

            Statement::DropEntity(ir) => {
                let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                Ok(RenderedOutput::Spreadsheet(SpreadsheetOutput {
                    operation: SpreadsheetOp::DropSheet {
                        if_exists: ir.if_exists,
                    },
                    sheet,
                    workbook: None,
                }))
            }

            Statement::AlterEntity(ir) => {
                // Only rename is naturally supported in spreadsheets.
                let rename_action = ir.actions.iter().find_map(|a| {
                    if let dol_core::ir::AlterAction::RenameEntity(new_name) = a {
                        Some(new_name.clone())
                    } else {
                        None
                    }
                });

                match rename_action {
                    Some(new_name) => {
                        let sheet =
                            qualified_name(&ir.target.namespace, &ir.target.name);

                        Ok(RenderedOutput::Spreadsheet(SpreadsheetOutput {
                            operation: SpreadsheetOp::RenameSheet { new_name },
                            sheet,
                            workbook: None,
                        }))
                    }
                    None => Err(BackendError::Unsupported(
                        "SpreadsheetBackend only supports RenameEntity alter actions".into(),
                    )),
                }
            }

            // ── Mutation ────────────────────────────────────────────────
            Statement::Insert(ir) => {
                let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                Ok(RenderedOutput::Spreadsheet(SpreadsheetOutput {
                    operation: SpreadsheetOp::AppendRows {
                        columns: ir.fields.clone(),
                        row_count: ir.row_count,
                    },
                    sheet,
                    workbook: None,
                }))
            }

            Statement::Update(ir) => {
                let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                let assignments = ir
                    .assignments
                    .iter()
                    .map(|(col, expr)| (col.clone(), render_expr_simple(expr)))
                    .collect();

                let filter = if ir.filters.is_empty() {
                    None
                } else {
                    Some(
                        ir.filters
                            .iter()
                            .map(render_expr_simple)
                            .collect::<Vec<_>>()
                            .join(" AND "),
                    )
                };

                Ok(RenderedOutput::Spreadsheet(SpreadsheetOutput {
                    operation: SpreadsheetOp::UpdateRows {
                        assignments,
                        filter,
                    },
                    sheet,
                    workbook: None,
                }))
            }

            Statement::Remove(ir) => {
                let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                let filter = if ir.filters.is_empty() {
                    None
                } else {
                    Some(
                        ir.filters
                            .iter()
                            .map(render_expr_simple)
                            .collect::<Vec<_>>()
                            .join(" AND "),
                    )
                };

                Ok(RenderedOutput::Spreadsheet(SpreadsheetOutput {
                    operation: SpreadsheetOp::DeleteRows { filter },
                    sheet,
                    workbook: None,
                }))
            }

            // ── Query ───────────────────────────────────────────────────
            Statement::Query(ir) => {
                let sheet = qualified_name(&ir.source.namespace, &ir.source.name);

                let columns = ir
                    .projections
                    .iter()
                    .map(render_expr_simple)
                    .collect();

                let filter = if ir.filters.is_empty() {
                    None
                } else {
                    Some(
                        ir.filters
                            .iter()
                            .map(render_expr_simple)
                            .collect::<Vec<_>>()
                            .join(" AND "),
                    )
                };

                let sort = ir
                    .order_by
                    .iter()
                    .map(render_sort_spec)
                    .collect();

                let limit = ir.limit.as_ref().and_then(|l| match l {
                    dol_core::ir::OffsetLimit::Value(v) => Some(*v),
                    dol_core::ir::OffsetLimit::Param => None,
                });

                let offset = ir.offset.as_ref().and_then(|o| match o {
                    dol_core::ir::OffsetLimit::Value(v) => Some(*v),
                    dol_core::ir::OffsetLimit::Param => None,
                });

                Ok(RenderedOutput::Spreadsheet(SpreadsheetOutput {
                    operation: SpreadsheetOp::ReadRows {
                        columns,
                        filter,
                        sort,
                        limit,
                        offset,
                        distinct: ir.distinct,
                    },
                    sheet,
                    workbook: None,
                }))
            }

            // ── Unsupported ─────────────────────────────────────────────
            Statement::InsertSelect(_) => Err(BackendError::Unsupported(
                "SpreadsheetBackend does not support INSERT ... SELECT".into(),
            )),
            Statement::Upsert(_) => Err(BackendError::Unsupported(
                "SpreadsheetBackend does not support UPSERT".into(),
            )),
            Statement::Compound(_) => Err(BackendError::Unsupported(
                "SpreadsheetBackend does not support compound queries (UNION/INTERSECT/EXCEPT)"
                    .into(),
            )),
            Statement::DefineIndex(_) | Statement::DropIndex(_) => Err(
                BackendError::Unsupported("Spreadsheets do not support indexes".into()),
            ),
            Statement::DefineType(_) | Statement::DropType(_) => Err(
                BackendError::Unsupported("Spreadsheets do not support custom types".into()),
            ),
            Statement::Grant(_) | Statement::Revoke(_) | Statement::DefinePolicy(_) => {
                Err(BackendError::Unsupported(
                    "Spreadsheets do not support access control".into(),
                ))
            }
            Statement::Transaction(_) => Err(BackendError::Unsupported(
                "Spreadsheets do not support transactions".into(),
            )),
            Statement::PutObject(_)
            | Statement::GetObject(_)
            | Statement::ListObjects(_)
            | Statement::ReadFile(_)
            | Statement::WriteFile(_)
            | Statement::MoveFile(_) => Err(BackendError::Unsupported(
                "SpreadsheetBackend does not support storage/file operations".into(),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build a qualified sheet name from optional namespace + name
// ---------------------------------------------------------------------------

fn qualified_name(namespace: &Option<String>, name: &str) -> String {
    match namespace {
        Some(ns) => format!("{}.{}", ns, name),
        None => name.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Simplified expression renderer for spreadsheet filter descriptions
// ---------------------------------------------------------------------------

/// Render an expression into a simple, human-readable string.
///
/// This is intentionally simplified — spreadsheets don't execute SQL, so we
/// produce a readable description rather than executable syntax.
fn render_expr_simple(expr: &Expr<'_>) -> String {
    match expr {
        Expr::Identifier(name) => name.to_string(),
        Expr::QualifiedIdentifier { scope, name } => format!("{}.{}", scope, name),
        Expr::FieldAccess { base, field } => {
            format!("{}.{}", render_expr_simple(base), field)
        }
        Expr::Param => "?".to_string(),
        Expr::Value(lit) => format!("{:?}", lit),
        Expr::BinaryOp {
            left,
            op,
            right,
            negated,
        } => {
            let op_str = match op {
                BinOp::Eq => "=",
                BinOp::Ne => "!=",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::And => "AND",
                BinOp::Or => "OR",
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Like => "LIKE",
                BinOp::ILike => "ILIKE",
                BinOp::Concat => "||",
                _ => "??",
            };
            let not_prefix = if *negated { "NOT " } else { "" };
            format!(
                "({}{} {} {})",
                not_prefix,
                render_expr_simple(left),
                op_str,
                render_expr_simple(right),
            )
        }
        Expr::UnaryOp { op, expr: inner } => {
            let op_str = match op {
                UnaryOp::Not => "NOT",
                UnaryOp::Neg => "-",
                UnaryOp::IsNull => "IS NULL",
                UnaryOp::IsNotNull => "IS NOT NULL",
                _ => "??",
            };
            match op {
                UnaryOp::IsNull | UnaryOp::IsNotNull => {
                    format!("({} {})", render_expr_simple(inner), op_str)
                }
                _ => format!("({} {})", op_str, render_expr_simple(inner)),
            }
        }
        Expr::Func { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(render_expr_simple).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        Expr::Cast { expr: inner, as_type } => {
            format!("CAST({} AS {})", render_expr_simple(inner), as_type)
        }
        Expr::Between {
            expr: inner,
            low,
            high,
            negated,
        } => {
            let not = if *negated { "NOT " } else { "" };
            format!(
                "({} {}BETWEEN {} AND {})",
                render_expr_simple(inner),
                not,
                render_expr_simple(low),
                render_expr_simple(high),
            )
        }
        Expr::InList {
            expr: inner,
            list,
            negated,
        } => {
            let not = if *negated { "NOT " } else { "" };
            let items: Vec<String> = list.iter().map(render_expr_simple).collect();
            format!(
                "({} {}IN ({}))",
                render_expr_simple(inner),
                not,
                items.join(", "),
            )
        }
        Expr::IsNull { expr: inner, negated } => {
            if *negated {
                format!("({} IS NOT NULL)", render_expr_simple(inner))
            } else {
                format!("({} IS NULL)", render_expr_simple(inner))
            }
        }
        Expr::Star => "*".to_string(),
        Expr::CountStar => "COUNT(*)".to_string(),
        Expr::Alias { expr: inner, alias } => {
            format!("{} AS {}", render_expr_simple(inner), alias)
        }
        Expr::Raw(s) => s.clone(),
        // For anything complex (subqueries, window functions, etc.), produce
        // a placeholder — these aren't meaningful in spreadsheet context.
        _ => "<expr>".to_string(),
    }
}

/// Convert an `OrderByExpr` to a `SpreadsheetSortSpec`.
fn render_sort_spec(order: &OrderByExpr<'_>) -> SpreadsheetSortSpec {
    let column = render_expr_simple(&order.expr);
    let direction = if order.direction == Direction::Desc {
        SortDirection::Descending
    } else {
        SortDirection::Ascending
    };
    SpreadsheetSortSpec { column, direction }
}

#[cfg(test)]
mod tests;
