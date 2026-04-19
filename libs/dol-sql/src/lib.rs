//! # dol-sql — DOL SQL Backend

#![deny(unsafe_code)]

pub mod dialect;
pub mod ext;
pub mod render;

use dialect::Dialect;
use dol_core::ir::*;

/// SQL output: rendered SQL text plus the number of bind parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SqlOutput {
    pub sql: String,
    pub param_count: usize,
}

/// SQL backend: renders DOL IR into dialect-specific SQL strings.
pub struct SqlBackend {
    pub dialect: Dialect,
}

impl SqlBackend {
    pub fn new(dialect: Dialect) -> Self { Self { dialect } }
    pub fn postgres() -> Self { Self::new(Dialect::postgres()) }
    pub fn mysql() -> Self { Self::new(Dialect::mysql()) }
    pub fn sqlite() -> Self { Self::new(Dialect::sqlite()) }
    pub fn mssql() -> Self { Self::new(Dialect::mssql()) }
    pub fn oracle() -> Self { Self::new(Dialect::oracle()) }

    pub fn render_query(&self, ir: &QueryIR) -> Result<SqlOutput, BackendError> {
        render::render_query_ir(ir, &self.dialect)
    }
    pub fn render_insert(&self, ir: &InsertIR) -> Result<SqlOutput, BackendError> {
        render::render_insert_ir(ir, &self.dialect)
    }
    pub fn render_insert_select(&self, ir: &InsertSelectIR) -> Result<SqlOutput, BackendError> {
        render::render_insert_select_ir(ir, &self.dialect)
    }
    pub fn render_update(&self, ir: &UpdateIR) -> Result<SqlOutput, BackendError> {
        render::render_update_ir(ir, &self.dialect)
    }
    pub fn render_remove(&self, ir: &RemoveIR) -> Result<SqlOutput, BackendError> {
        render::render_remove_ir(ir, &self.dialect)
    }
    pub fn render_upsert(&self, ir: &UpsertIR) -> Result<SqlOutput, BackendError> {
        render::render_upsert_ir(ir, &self.dialect)
    }
    pub fn render_define_entity(&self, ir: &DefineEntityIR) -> Result<SqlOutput, BackendError> {
        render::render_define_entity_ir(ir, &self.dialect)
    }
    pub fn render_alter_entity(&self, ir: &AlterEntityIR) -> Result<SqlOutput, BackendError> {
        render::render_alter_entity_ir(ir, &self.dialect)
    }
    pub fn render_drop_entity(&self, ir: &DropEntityIR) -> Result<SqlOutput, BackendError> {
        render::render_drop_entity_ir(ir, &self.dialect)
    }
    pub fn render_define_index(&self, ir: &DefineIndexIR) -> Result<SqlOutput, BackendError> {
        render::render_define_index_ir(ir, &self.dialect)
    }
    pub fn render_compound(&self, ir: &CompoundQueryIR) -> Result<SqlOutput, BackendError> {
        render::render_compound_query_ir(ir, &self.dialect)
    }
    pub fn render_drop_index(&self, ir: &DropIndexIR) -> Result<SqlOutput, BackendError> {
        render::render_drop_index_ir(ir, &self.dialect)
    }
    pub fn render_grant(&self, ir: &GrantIR) -> Result<SqlOutput, BackendError> {
        render::render_grant_ir(ir)
    }
    pub fn render_revoke(&self, ir: &RevokeIR) -> Result<SqlOutput, BackendError> {
        render::render_revoke_ir(ir)
    }
    pub fn render_transaction(&self, ir: &TransactionIR) -> Result<SqlOutput, BackendError> {
        render::render_transaction_ir(ir, &self.dialect)
    }
    pub fn render_define_type(&self, ir: &DefineTypeIR) -> Result<SqlOutput, BackendError> {
        render::render_define_type_ir(ir, &self.dialect)
    }
    pub fn render_drop_type(&self, ir: &DropTypeIR) -> Result<SqlOutput, BackendError> {
        render::render_drop_type_ir(ir, &self.dialect)
    }
    pub fn render_define_policy(&self, ir: &DefinePolicyIR) -> Result<SqlOutput, BackendError> {
        render::render_define_policy_ir(ir, &self.dialect)
    }

    /// Render a Statement to SqlOutput (all SQL-supported variants).
    pub fn render(&self, stmt: &Statement) -> Result<SqlOutput, BackendError> {
        match stmt {
            Statement::Query(ir) => self.render_query(ir),
            Statement::Insert(ir) => self.render_insert(ir),
            Statement::InsertSelect(ir) => self.render_insert_select(ir),
            Statement::Update(ir) => self.render_update(ir),
            Statement::Remove(ir) => self.render_remove(ir),
            Statement::Upsert(ir) => self.render_upsert(ir),
            Statement::DefineEntity(ir) => self.render_define_entity(ir),
            Statement::AlterEntity(ir) => self.render_alter_entity(ir),
            Statement::DropEntity(ir) => self.render_drop_entity(ir),
            Statement::DefineIndex(ir) => self.render_define_index(ir),
            Statement::DropIndex(ir) => self.render_drop_index(ir),
            Statement::Grant(ir) => self.render_grant(ir),
            Statement::Revoke(ir) => self.render_revoke(ir),
            Statement::Transaction(ir) => self.render_transaction(ir),
            Statement::DefineType(ir) => self.render_define_type(ir),
            Statement::DropType(ir) => self.render_drop_type(ir),
            Statement::Compound(ir) => self.render_compound(ir),
            Statement::DefinePolicy(ir) => self.render_define_policy(ir),
            Statement::PutObject(_)
            | Statement::GetObject(_)
            | Statement::ListObjects(_)
            | Statement::ReadFile(_)
            | Statement::WriteFile(_)
            | Statement::MoveFile(_) => Err(BackendError::Unsupported(
                "storage operations have no SQL equivalent".into(),
            )),
        }
    }
}
