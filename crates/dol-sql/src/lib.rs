//! # dol-sql — DOL SQL Backend
//!
//! Dialect-aware SQL rendering engine for DOL. Renders DOL IR into SQL strings
//! targeting PostgreSQL, MySQL, SQLite, MSSQL, Oracle, CockroachDB, and more.

pub mod dialect;
pub mod ext;
pub mod render;

use dialect::Dialect;
use dol_ir::*;

/// SQL backend: renders DOL IR into dialect-specific SQL strings.
pub struct SqlBackend {
    pub dialect: Dialect,
}

impl SqlBackend {
    pub fn new(dialect: Dialect) -> Self {
        Self { dialect }
    }

    pub fn postgres() -> Self {
        Self::new(Dialect::postgres())
    }

    pub fn mysql() -> Self {
        Self::new(Dialect::mysql())
    }

    pub fn sqlite() -> Self {
        Self::new(Dialect::sqlite())
    }

    pub fn mssql() -> Self {
        Self::new(Dialect::mssql())
    }

    pub fn oracle() -> Self {
        Self::new(Dialect::oracle())
    }

    /// Render a QueryIR to SQL.
    pub fn render_query(&self, ir: &QueryIR) -> Result<SqlOutput, BackendError> {
        render::render_query_ir(ir, &self.dialect)
    }

    /// Render an InsertIR to SQL.
    pub fn render_insert(&self, ir: &InsertIR) -> Result<SqlOutput, BackendError> {
        render::render_insert_ir(ir, &self.dialect)
    }

    /// Render an InsertSelectIR to SQL.
    pub fn render_insert_select(&self, ir: &InsertSelectIR) -> Result<SqlOutput, BackendError> {
        render::render_insert_select_ir(ir, &self.dialect)
    }

    /// Render an UpdateIR to SQL.
    pub fn render_update(&self, ir: &UpdateIR) -> Result<SqlOutput, BackendError> {
        render::render_update_ir(ir, &self.dialect)
    }

    /// Render a RemoveIR to SQL.
    pub fn render_remove(&self, ir: &RemoveIR) -> Result<SqlOutput, BackendError> {
        render::render_remove_ir(ir, &self.dialect)
    }

    /// Render an UpsertIR to SQL.
    pub fn render_upsert(&self, ir: &UpsertIR) -> Result<SqlOutput, BackendError> {
        render::render_upsert_ir(ir, &self.dialect)
    }

    /// Render a DefineModelIR to SQL.
    pub fn render_define_model(&self, ir: &DefineModelIR) -> Result<SqlOutput, BackendError> {
        render::render_define_model_ir(ir, &self.dialect)
    }

    /// Render an AlterModelIR to SQL.
    pub fn render_alter_model(&self, ir: &AlterModelIR) -> Result<SqlOutput, BackendError> {
        render::render_alter_model_ir(ir, &self.dialect)
    }

    /// Render a DropModelIR to SQL.
    pub fn render_drop_model(&self, ir: &DropModelIR) -> Result<SqlOutput, BackendError> {
        render::render_drop_model_ir(ir, &self.dialect)
    }

    /// Render a DefineIndexIR to SQL.
    pub fn render_define_index(&self, ir: &DefineIndexIR) -> Result<SqlOutput, BackendError> {
        render::render_define_index_ir(ir, &self.dialect)
    }

    /// Render a CompoundQueryIR to SQL.
    pub fn render_compound(&self, ir: &CompoundQueryIR) -> Result<SqlOutput, BackendError> {
        render::render_compound_query_ir(ir, &self.dialect)
    }

    /// Render a DropIndexIR to SQL.
    pub fn render_drop_index(&self, ir: &DropIndexIR) -> Result<SqlOutput, BackendError> {
        render::render_drop_index_ir(ir, &self.dialect)
    }

    /// Render a GrantIR to SQL.
    pub fn render_grant(&self, ir: &GrantIR) -> Result<SqlOutput, BackendError> {
        render::render_grant_ir(ir)
    }

    /// Render a RevokeIR to SQL.
    pub fn render_revoke(&self, ir: &RevokeIR) -> Result<SqlOutput, BackendError> {
        render::render_revoke_ir(ir)
    }

    /// Render a TransactionIR to SQL.
    pub fn render_transaction(&self, ir: &TransactionIR) -> Result<SqlOutput, BackendError> {
        render::render_transaction_ir(ir)
    }
}

impl Backend for SqlBackend {
    fn render(&self, stmt: &Statement) -> Result<RenderedOutput, BackendError> {
        let output = match stmt {
            Statement::Query(ir) => self.render_query(ir)?,
            Statement::Insert(ir) => self.render_insert(ir)?,
            Statement::InsertSelect(ir) => self.render_insert_select(ir)?,
            Statement::Update(ir) => self.render_update(ir)?,
            Statement::Remove(ir) => self.render_remove(ir)?,
            Statement::Upsert(ir) => self.render_upsert(ir)?,
            Statement::DefineModel(ir) => self.render_define_model(ir)?,
            Statement::AlterModel(ir) => self.render_alter_model(ir)?,
            Statement::DropModel(ir) => self.render_drop_model(ir)?,
            Statement::DefineIndex(ir) => self.render_define_index(ir)?,
            Statement::DropIndex(ir) => self.render_drop_index(ir)?,
            Statement::Grant(ir) => self.render_grant(ir)?,
            Statement::Revoke(ir) => self.render_revoke(ir)?,
            Statement::Transaction(ir) => self.render_transaction(ir)?,
            Statement::DefineType(_) => {
                return Err(BackendError::Unsupported(
                    "DefineType not yet implemented".into(),
                ))
            }
            Statement::DropType(_) => {
                return Err(BackendError::Unsupported(
                    "DropType not yet implemented".into(),
                ))
            }
            Statement::Compound(ir) => self.render_compound(ir)?,
            Statement::DefinePolicy(_) => {
                return Err(BackendError::Unsupported(
                    "DefinePolicy not yet implemented".into(),
                ))
            }
            Statement::PutObject(_)
            | Statement::GetObject(_)
            | Statement::ListObjects(_)
            | Statement::ReadFile(_)
            | Statement::WriteFile(_)
            | Statement::MoveFile(_) => {
                return Err(BackendError::Unsupported(
                    "storage operations have no SQL equivalent".into(),
                ));
            }
        };
        Ok(RenderedOutput::Sql(output))
    }
}
