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

use dol_core::expr::{Direction, Expr, Literal, OpId, OrderByExpr, UnaryOp};
use dol_core::ir::Statement;

/// A column definition for spreadsheet sheet creation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpreadsheetColumnDef {
    pub name: String,
    pub data_type: dol_core::types::DataType,
}

/// A sort specification for spreadsheet read operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpreadsheetSortSpec {
    pub column: String,
    pub direction: Direction,
}

/// A spreadsheet operation descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpreadsheetOutput {
    pub operation: SpreadsheetOp,
    pub sheet: String,
    pub workbook: Option<String>,
}

/// Spreadsheet operation kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpreadsheetOp {
    CreateSheet {
        columns: Vec<SpreadsheetColumnDef>,
        if_not_exists: bool,
    },
    AppendRows {
        columns: Vec<String>,
        row_count: usize,
    },
    UpdateRows {
        assignments: Vec<(String, String)>,
        filter: Option<String>,
    },
    DeleteRows {
        filter: Option<String>,
    },
    ReadRows {
        columns: Vec<String>,
        filter: Option<String>,
        sort: Vec<SpreadsheetSortSpec>,
        limit: Option<u64>,
        offset: Option<u64>,
        distinct: bool,
    },
    RenameSheet {
        new_name: String,
    },
    DropSheet {
        if_exists: bool,
    },
}

use dol_core::ir::BackendError;

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

impl SpreadsheetBackend {
    pub fn render(&self, stmt: &Statement<'_>) -> Result<SpreadsheetOutput, BackendError> {
        match stmt {
            // ── Definition ──────────────────────────────────────────────
            Statement::DefineEntity(ir) => {
                // Reject table-level constraints (spreadsheets have no constraint system)
                if !ir.constraints.is_empty() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support table-level constraints".into(),
                    ));
                }

                // Reject unsupported field-level attributes
                for f in &ir.fields {
                    if f.primary_key {
                        return Err(BackendError::Unsupported(format!(
                            "SpreadsheetBackend does not support primary_key (field '{}')",
                            f.name,
                        )));
                    }
                    if f.unique {
                        return Err(BackendError::Unsupported(format!(
                            "SpreadsheetBackend does not support unique constraint (field '{}')",
                            f.name,
                        )));
                    }
                    if f.references.is_some() {
                        return Err(BackendError::Unsupported(format!(
                            "SpreadsheetBackend does not support foreign key references (field '{}')",
                            f.name,
                        )));
                    }
                    if f.check.is_some() {
                        return Err(BackendError::Unsupported(format!(
                            "SpreadsheetBackend does not support check constraints (field '{}')",
                            f.name,
                        )));
                    }
                    if f.generated.is_some() {
                        return Err(BackendError::Unsupported(format!(
                            "SpreadsheetBackend does not support generated columns (field '{}')",
                            f.name,
                        )));
                    }
                    if f.auto_increment {
                        return Err(BackendError::Unsupported(format!(
                            "SpreadsheetBackend does not support auto_increment (field '{}')",
                            f.name,
                        )));
                    }
                }

                let columns = ir
                    .fields
                    .iter()
                    .map(|f| SpreadsheetColumnDef {
                        name: f.name.clone(),
                        data_type: f.data_type.clone(),
                    })
                    .collect();

                let sheet = qualified_name(&ir.namespace, &ir.name);

                Ok(SpreadsheetOutput {
                    operation: SpreadsheetOp::CreateSheet {
                        columns,
                        if_not_exists: ir.if_not_exists,
                    },
                    sheet,
                    workbook: None,
                })
            }

            Statement::DropEntity(ir) => {
                if ir.cascade {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support CASCADE drops".into(),
                    ));
                }

                let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                Ok(SpreadsheetOutput {
                    operation: SpreadsheetOp::DropSheet {
                        if_exists: ir.if_exists,
                    },
                    sheet,
                    workbook: None,
                })
            }

            Statement::AlterEntity(ir) => {
                // Only a single rename action is supported in spreadsheets.
                match ir.actions.as_slice() {
                    [dol_core::ir::AlterAction::RenameEntity(new_name)] => {
                        let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                        Ok(SpreadsheetOutput {
                            operation: SpreadsheetOp::RenameSheet {
                                new_name: qualified_name(&ir.target.namespace, new_name),
                            },
                            sheet,
                            workbook: None,
                        })
                    }
                    [] => Err(BackendError::Unsupported(
                        "SpreadsheetBackend requires exactly one AlterEntity action, \
                         and it must be RenameEntity; received no actions"
                            .into(),
                    )),
                    [_] => Err(BackendError::Unsupported(
                        "SpreadsheetBackend only supports AlterEntity with exactly \
                         one RenameEntity action; received a different single action"
                            .into(),
                    )),
                    actions => Err(BackendError::Unsupported(format!(
                        "SpreadsheetBackend only supports AlterEntity with exactly \
                             one RenameEntity action; rejected {} actions",
                        actions.len()
                    ))),
                }
            }

            // ── Mutation ────────────────────────────────────────────────
            Statement::Insert(ir) => {
                if !ir.returning.is_empty() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support RETURNING clauses on INSERT".into(),
                    ));
                }

                let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                Ok(SpreadsheetOutput {
                    operation: SpreadsheetOp::AppendRows {
                        columns: ir.fields.clone(),
                        row_count: ir.row_count,
                    },
                    sheet,
                    workbook: None,
                })
            }

            Statement::Update(ir) => {
                if !ir.returning.is_empty() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support RETURNING clauses on UPDATE".into(),
                    ));
                }

                let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                let assignments = ir
                    .assignments
                    .iter()
                    .map(|(col, expr)| Ok((col.clone(), render_expr_simple(expr)?)))
                    .collect::<Result<Vec<_>, BackendError>>()?;

                let filter = render_filter_list(&ir.filters)?;

                Ok(SpreadsheetOutput {
                    operation: SpreadsheetOp::UpdateRows {
                        assignments,
                        filter,
                    },
                    sheet,
                    workbook: None,
                })
            }

            Statement::Remove(ir) => {
                if !ir.returning.is_empty() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support RETURNING clauses on DELETE".into(),
                    ));
                }

                let sheet = qualified_name(&ir.target.namespace, &ir.target.name);

                let filter = render_filter_list(&ir.filters)?;

                Ok(SpreadsheetOutput {
                    operation: SpreadsheetOp::DeleteRows { filter },
                    sheet,
                    workbook: None,
                })
            }

            // ── Query ───────────────────────────────────────────────────
            Statement::Query(ir) => {
                // Reject unsupported query features
                if !ir.joins.is_empty() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support JOINs".into(),
                    ));
                }
                if !ir.group_by.is_empty() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support GROUP BY".into(),
                    ));
                }
                if !ir.having.is_empty() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support HAVING".into(),
                    ));
                }
                if !ir.distinct_on.is_empty() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support DISTINCT ON".into(),
                    ));
                }
                if ir.lock_mode.is_some() {
                    return Err(BackendError::Unsupported(
                        "SpreadsheetBackend does not support lock modes (FOR UPDATE/SHARE)".into(),
                    ));
                }

                let sheet = qualified_name(&ir.source.namespace, &ir.source.name);

                let columns = ir
                    .projections
                    .iter()
                    .map(|e| validate_column_expr(e, "projection"))
                    .collect::<Result<Vec<_>, BackendError>>()?;

                let filter = render_filter_list(&ir.filters)?;

                let sort = ir
                    .order_by
                    .iter()
                    .map(render_sort_spec)
                    .collect::<Result<Vec<_>, BackendError>>()?;

                let limit = match &ir.limit {
                    Some(dol_core::ir::OffsetLimit::Value(v)) => Some(*v),
                    Some(dol_core::ir::OffsetLimit::Param) => {
                        // Reject parameterized LIMIT here explicitly, matching
                        // the OFFSET handling below.
                        return Err(BackendError::Unsupported(
                            "SpreadsheetBackend does not support parameterized LIMIT".into(),
                        ));
                    }
                    None => None,
                };

                let offset = match &ir.offset {
                    Some(dol_core::ir::OffsetLimit::Value(v)) => Some(*v),
                    Some(dol_core::ir::OffsetLimit::Param) => {
                        return Err(BackendError::Unsupported(
                            "SpreadsheetBackend does not support parameterized OFFSET".into(),
                        ));
                    }
                    None => None,
                };

                Ok(SpreadsheetOutput {
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
                })
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
            Statement::DefineIndex(_) | Statement::DropIndex(_) => Err(BackendError::Unsupported(
                "Spreadsheets do not support indexes".into(),
            )),
            Statement::DefineType(_) | Statement::DropType(_) => Err(BackendError::Unsupported(
                "Spreadsheets do not support custom types".into(),
            )),
            Statement::Grant(_) | Statement::Revoke(_) | Statement::DefinePolicy(_) => Err(
                BackendError::Unsupported("Spreadsheets do not support access control".into()),
            ),
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

/// Render a list of filter expressions joined by AND, returning `None` if empty.
fn render_filter_list(filters: &[Expr<'_>]) -> Result<Option<String>, BackendError> {
    if filters.is_empty() {
        return Ok(None);
    }
    let parts = filters
        .iter()
        .map(render_expr_simple)
        .collect::<Result<Vec<_>, BackendError>>()?;
    Ok(Some(parts.join(" AND ")))
}

/// Render a literal value into a deterministic, human-readable string suitable
/// for spreadsheet operation descriptors.
///
/// Uses stable formatting rather than `Debug`, so that serialized
/// `SpreadsheetOp` values are predictable and executor-friendly.
fn render_literal(lit: &Literal<'_>) -> Result<String, BackendError> {
    use dol_core::expr::Literal as L;
    match lit {
        L::Null => Ok("NULL".to_string()),
        L::Bool(v) => Ok(if *v { "true" } else { "false" }.to_string()),
        L::String(v) => Ok(format!("'{}'", v.replace('\'', "''"))),
        L::Json(v) => Ok(format!("'{}'", v)),
        L::Xml(v) => Ok(format!("'{}'", v)),
        L::Enum(v) => Ok(format!("'{}'", v)),
        L::Bytes(v) => {
            let hex: String = v.iter().map(|b| format!("{:02x}", b)).collect();
            Ok(format!("0x{}", hex))
        }
        L::Uuid(v) => {
            let u = uuid_from_bytes(v);
            Ok(format!("'{}'", u))
        }
        L::Int8(v) => Ok(v.to_string()),
        L::Int16(v) => Ok(v.to_string()),
        L::Int32(v) => Ok(v.to_string()),
        L::Int64(v) => Ok(v.to_string()),
        L::Int128(v) => Ok(v.to_string()),
        L::UInt8(v) => Ok(v.to_string()),
        L::UInt16(v) => Ok(v.to_string()),
        L::UInt32(v) => Ok(v.to_string()),
        L::UInt64(v) => Ok(v.to_string()),
        L::UInt128(v) => Ok(v.to_string()),
        L::Float32(v) => Ok(v.to_string()),
        L::Float64(v) => Ok(v.to_string()),
        L::Decimal(v) => Ok(v.to_string()),
        L::Inet(v) => Ok(format!("'{}'", v)),
        L::MacAddr(v) => Ok(format!("'{}'", v)),
        L::MacAddr8(v) => Ok(format!("'{}'", v)),
        L::Date(v) => Ok(format!("'{}'", v)),
        L::Time(v) => Ok(format!("'{}'", v)),
        L::DateTime(v) => Ok(format!("'{}'", v)),
        L::TimestampTz(v) => Ok(format!("'{}'", v)),
        L::Interval(v) => Ok(format!("'{}'", v)),
        L::BitString(v) => Ok(format!("'{}'", v)),
        // Geometric types
        L::Point(v) => Ok(format!("'{}'", v)),
        L::Line(v) => Ok(format!("'{}'", v)),
        L::Segment(v) => Ok(format!("'{}'", v)),
        L::Rect(v) => Ok(format!("'{}'", v)),
        L::Circle(v) => Ok(format!("'{}'", v)),
        L::Path(v) => Ok(format!("'{}'", v)),
        L::Polygon(v) => Ok(format!("'{}'", v)),
        // Composite types are not supported in spreadsheet descriptors.
        L::Array(_) | L::Set(_) | L::Tuple(_) | L::Map(_) | L::Struct(_) | L::Range(_) => {
            Err(BackendError::Unsupported(
                "SpreadsheetBackend does not support composite literal types (array, set, tuple, map, struct, range)".into(),
            ))
        }
        // Extension types are not supported.
        L::Extension { .. } => {
            Err(BackendError::Unsupported(
                "SpreadsheetBackend does not support extension literal types".into(),
            ))
        }
    }
}

/// Format a UUID byte array as a canonical hyphenated string.
fn uuid_from_bytes(b: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Render an expression into a simple, human-readable string.
///
/// Returns `Err(BackendError::Unsupported)` for expression variants that
/// have no meaningful spreadsheet representation (subqueries, window
/// functions, quantified comparisons, etc.).
fn render_expr_simple(expr: &Expr<'_>) -> Result<String, BackendError> {
    match expr {
        Expr::Identifier(name) => Ok(name.to_string()),
        Expr::QualifiedIdentifier { scope, name } => Ok(format!("{}.{}", scope, name)),
        Expr::FieldAccess { base, field } => Ok(format!("{}.{}", render_expr_simple(base)?, field)),
        Expr::Param => Ok("?".to_string()),
        Expr::Value(lit) => render_literal(lit),
        Expr::BinaryOp {
            left,
            op,
            right,
            negated,
        } => {
            let op_str = match op.as_str() {
                OpId::EQ => "=",
                OpId::NE => "!=",
                OpId::LT => "<",
                OpId::LE => "<=",
                OpId::GT => ">",
                OpId::GE => ">=",
                OpId::AND => "AND",
                OpId::OR => "OR",
                OpId::ADD => "+",
                OpId::SUB => "-",
                OpId::MUL => "*",
                OpId::DIV => "/",
                OpId::MOD => "%",
                OpId::LIKE => "LIKE",
                OpId::ILIKE => "ILIKE",
                OpId::CONCAT => "||",
                other => {
                    return Err(BackendError::Unsupported(format!(
                        "SpreadsheetBackend does not support binary operator '{}'",
                        other,
                    )));
                }
            };
            let base_expr = format!(
                "({} {} {})",
                render_expr_simple(left)?,
                op_str,
                render_expr_simple(right)?,
            );
            if *negated {
                Ok(format!("(NOT {})", base_expr))
            } else {
                Ok(base_expr)
            }
        }
        Expr::UnaryOp { op, expr: inner } => {
            let op_str = match op {
                UnaryOp::Not => "NOT",
                UnaryOp::Neg => "-",
                UnaryOp::BitNot => "~",
            };
            Ok(format!("({} {})", op_str, render_expr_simple(inner)?))
        }
        Expr::Func { name, args } => {
            let arg_strs = args
                .iter()
                .map(render_expr_simple)
                .collect::<Result<Vec<_>, BackendError>>()?;
            let func_str = spreadsheet_func_name(name);
            Ok(format!("{}({})", func_str, arg_strs.join(", ")))
        }
        Expr::Cast {
            expr: inner,
            as_type,
        } => Ok(format!(
            "CAST({} AS {})",
            render_expr_simple(inner)?,
            render_spreadsheet_type(as_type)
        )),
        Expr::Between {
            expr: inner,
            low,
            high,
            negated,
        } => {
            let not = if *negated { "NOT " } else { "" };
            Ok(format!(
                "({} {}BETWEEN {} AND {})",
                render_expr_simple(inner)?,
                not,
                render_expr_simple(low)?,
                render_expr_simple(high)?,
            ))
        }
        Expr::InList {
            expr: inner,
            list,
            negated,
        } => {
            let not = if *negated { "NOT " } else { "" };
            let items = list
                .iter()
                .map(render_expr_simple)
                .collect::<Result<Vec<_>, BackendError>>()?;
            Ok(format!(
                "({} {}IN ({}))",
                render_expr_simple(inner)?,
                not,
                items.join(", "),
            ))
        }
        Expr::IsNull {
            expr: inner,
            negated,
        } => {
            if *negated {
                Ok(format!("({} IS NOT NULL)", render_expr_simple(inner)?))
            } else {
                Ok(format!("({} IS NULL)", render_expr_simple(inner)?))
            }
        }
        Expr::Star => Ok("*".to_string()),
        Expr::CountStar => Ok("COUNT(*)".to_string()),
        Expr::Alias { expr: inner, alias } => {
            Ok(format!("{} AS {}", render_expr_simple(inner)?, alias))
        }
        Expr::Raw(_) => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support raw expressions".into(),
        )),

        // ── Unsupported expression variants ─────────────────────────
        Expr::Subquery(_) => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support subquery expressions".into(),
        )),
        Expr::InSubquery { .. } => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support IN (subquery) expressions".into(),
        )),
        Expr::Exists { .. } => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support EXISTS expressions".into(),
        )),
        Expr::Window { .. } => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support window function expressions".into(),
        )),
        Expr::QuantifiedCmp { .. } => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support quantified comparisons (ANY/ALL)".into(),
        )),
        Expr::Case { .. } => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support CASE expressions".into(),
        )),
        Expr::ObjectLiteral(_) => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support object literal expressions".into(),
        )),
        Expr::ArrayLiteral(_) => Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support array literal expressions".into(),
        )),
    }
}

/// Convert an `OrderByExpr` to a `SpreadsheetSortSpec`.
fn render_sort_spec(order: &OrderByExpr<'_>) -> Result<SpreadsheetSortSpec, BackendError> {
    if order.nulls.is_some() {
        return Err(BackendError::Unsupported(
            "SpreadsheetBackend does not support NULLS FIRST/LAST ordering".into(),
        ));
    }
    let column = validate_column_expr(&order.expr, "ORDER BY")?;
    Ok(SpreadsheetSortSpec {
        column,
        direction: order.direction,
    })
}

/// Validate that an expression is a supported column expression, returning the
/// rendered name.
///
/// Accepted forms are identifiers, qualified identifiers, `*`, and `COUNT(*)`.
///
/// SpreadsheetOp::ReadRows `columns` and `SpreadsheetSortSpec::column` are
/// documented as source column names, so aliases and other expression kinds are
/// rejected because they would lose the underlying sheet column identity.
fn validate_column_expr(expr: &Expr<'_>, context: &str) -> Result<String, BackendError> {
    match expr {
        Expr::Identifier(name) => Ok(name.to_string()),
        Expr::QualifiedIdentifier { scope, name } => Ok(format!("{}.{}", scope, name)),
        Expr::Star => Ok("*".to_string()),
        Expr::Alias { alias, .. } => Err(BackendError::Unsupported(format!(
            "SpreadsheetBackend does not support aliased column expressions in {}: alias '{}' \
             would hide the underlying source column name",
            context, alias,
        ))),
        Expr::CountStar => Ok("COUNT(*)".to_string()),
        other => Err(BackendError::Unsupported(format!(
            "SpreadsheetBackend only supports column identifiers, qualified identifiers, \
             *, and COUNT(*) in {}; got unsupported expression: {:?}",
            context, other,
        ))),
    }
}

fn spreadsheet_func_name<'a>(name: &'a dol_core::expr::FuncId) -> std::borrow::Cow<'a, str> {
    use dol_core::expr::FuncId as K;
    match name.as_str() {
        // ── Renamed functions (DOL name differs from spreadsheet name) ───
        K::LENGTH => std::borrow::Cow::Borrowed("LEN"),
        K::NULLIF | K::IFNULL => std::borrow::Cow::Borrowed("IFERROR"),
        K::CURRENT_DATE => std::borrow::Cow::Borrowed("TODAY"),

        // ── Default: pass through as-is (already uppercase) ──────────────
        other => std::borrow::Cow::Borrowed(other),
    }
}

fn render_spreadsheet_type(dt: &dol_core::types::DataType) -> &'static str {
    use dol_core::types::DataType as D;
    match dt {
        D::Int8
        | D::Int16
        | D::Int32
        | D::Int64
        | D::Int128
        | D::UInt8
        | D::UInt16
        | D::UInt32
        | D::UInt64
        | D::UInt128 => "NUMBER",
        D::Float32 | D::Float64 | D::Decimal { .. } => "DECIMAL",
        D::Text | D::Varchar(_) | D::Char(_) => "TEXT",
        D::Bool => "BOOLEAN",
        D::Date => "DATE",
        D::Time { .. } | D::DateTime { .. } | D::TimestampTz { .. } => "DATETIME",
        _ => "TEXT",
    }
}

#[cfg(test)]
mod tests;
