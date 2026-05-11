//! [`Encode`] impls for `dol-command`, `dol-schema`, and the top-level [`Program`].

extern crate alloc;

use dol_command::operation::acl::{
    AuditEvent, AuditOp, AuditSink, Grant, MaskOp, PolicyOp, PolicyScope, QuotaKind, QuotaOp,
    Revoke,
};
use dol_command::operation::ddl::{
    FieldDef, FieldOp, IndexDirection, IndexKey, IndexMethod, IndexOp, LookupMethod, LookupOp,
    SchemaBody, SchemaOp,
};
use dol_command::operation::dml::{
    Append, Delete, Insert, InsertSource, Replace, ReplaceBody, Update, Upsert,
};
use dol_command::operation::dql::{Describe, DescribeFacet, Probe, Query};
use dol_command::operation::meta::{ExtensionId, OperationExtension};
use dol_command::operation::tx::{IsolationLevel, TxBegin, TxOp, TxOptions};
use dol_command::operation::{Operation, StructuralVerb};
use dol_command::privilege::Privilege;
use dol_command::program::Program;
use dol_command::target::{Locator, SchemaBinding, Symbol, Target, TargetKind};
use dol_core::policy::Budget;
use dol_schema::TypeBody;
use dol_schema::constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
use dol_schema::{CatalogEntry, CatalogId, SchemaCatalog, SchemaId, SchemaRef, TypeEntry};
use dol_schema::{Entity, Field};

use crate::encoder::{Encode, EncodeError, Writer, encode_slice};

// ─── Symbol ──────────────────────────────────────────────────────────────────

impl Encode for Symbol {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        self.0.encode(w, b)
    }
}

// ─── CatalogId / SchemaId / SchemaRef ────────────────────────────────────────

impl Encode for CatalogId {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        w.write_varint_u32(self.0)
    }
}

impl Encode for SchemaId {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        w.write_varint_u32(self.0)
    }
}

impl Encode for SchemaRef {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.catalog.encode(w, b))??;
        b.descend(|b| self.schema.encode(w, b))??;
        Ok(())
    }
}

// ─── Locator ─────────────────────────────────────────────────────────────────

impl Encode for Locator {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.namespace.encode(w, b))??;
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.path.encode(w, b))??;
        Ok(())
    }
}

// ─── TargetKind ──────────────────────────────────────────────────────────────

impl Encode for TargetKind {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            TargetKind::Relation => w.write_varint_u32(0),
            TargetKind::Document => w.write_varint_u32(1),
            TargetKind::KeyValue => w.write_varint_u32(2),
            TargetKind::Blob => w.write_varint_u32(3),
            TargetKind::FileTree => w.write_varint_u32(4),
            TargetKind::ApiResource => w.write_varint_u32(5),
            TargetKind::StreamTopic => w.write_varint_u32(6),
            TargetKind::Virtual => w.write_varint_u32(7),
            TargetKind::Custom(sym) => {
                w.write_varint_u32(8)?;
                b.descend(|b| sym.encode(w, b))??;
                Ok(())
            }
        }
    }
}

// ─── SchemaBinding ───────────────────────────────────────────────────────────

impl Encode for SchemaBinding {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            SchemaBinding::Declared(r) => {
                w.write_varint_u32(0)?;
                b.descend(|b| r.encode(w, b))??;
            }
            SchemaBinding::Inferred => w.write_varint_u32(1)?,
            SchemaBinding::Opaque => w.write_varint_u32(2)?,
        }
        Ok(())
    }
}

// ─── Target ──────────────────────────────────────────────────────────────────

impl Encode for Target {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.kind.encode(w, b))??;
        b.descend(|b| self.locator.encode(w, b))??;
        b.descend(|b| self.alias.encode(w, b))??;
        b.descend(|b| self.schema.encode(w, b))??;
        Ok(())
    }
}

// ─── Privilege ───────────────────────────────────────────────────────────────

impl Encode for Privilege {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            Privilege::Select => w.write_varint_u32(0),
            Privilege::Insert => w.write_varint_u32(1),
            Privilege::Update => w.write_varint_u32(2),
            Privilege::Delete => w.write_varint_u32(3),
            Privilege::All => w.write_varint_u32(4),
            Privilege::Usage => w.write_varint_u32(5),
            Privilege::Create => w.write_varint_u32(6),
            Privilege::Connect => w.write_varint_u32(7),
            Privilege::Custom(sym) => {
                w.write_varint_u32(8)?;
                b.descend(|b| sym.encode(w, b))??;
                Ok(())
            }
        }
    }
}

// ─── StructuralVerb ──────────────────────────────────────────────────────────

impl Encode for StructuralVerb {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            StructuralVerb::Create => w.write_varint_u32(0),
            StructuralVerb::Drop => w.write_varint_u32(1),
            StructuralVerb::Alter => w.write_varint_u32(2),
            StructuralVerb::Rename => w.write_varint_u32(3),
            StructuralVerb::Truncate => w.write_varint_u32(4),
        }
    }
}

// ─── TypeBody ────────────────────────────────────────────────────────────────

impl Encode for TypeBody {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            TypeBody::Enum => w.write_varint_u32(0),
            TypeBody::Composite => w.write_varint_u32(1),
            TypeBody::Distinct => w.write_varint_u32(2),
            TypeBody::Other => w.write_varint_u32(3),
        }
    }
}

// ─── SchemaBody ──────────────────────────────────────────────────────────────

impl Encode for SchemaBody {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            SchemaBody::Entity {
                schema,
                if_not_exists,
            } => {
                w.write_varint_u32(0)?;
                b.descend(|b| schema.encode(w, b))??;
                b.descend(|b| if_not_exists.encode(w, b))??;
            }
            SchemaBody::Type { schema, body } => {
                w.write_varint_u32(1)?;
                b.descend(|b| schema.encode(w, b))??;
                b.descend(|b| body.encode(w, b))??;
            }
            SchemaBody::Reference => w.write_varint_u32(2)?,
        }
        Ok(())
    }
}

// ─── SchemaOp ────────────────────────────────────────────────────────────────

impl Encode for SchemaOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.verb.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.body.encode(w, b))??;
        b.descend(|b| self.new_name.encode(w, b))??;
        Ok(())
    }
}

// ─── RefAction ───────────────────────────────────────────────────────────────

impl Encode for RefAction {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            RefAction::Forbid => w.write_varint_u32(0),
            RefAction::Cascade => w.write_varint_u32(1),
            RefAction::Detach => w.write_varint_u32(2),
            RefAction::Reject => w.write_varint_u32(3),
            RefAction::UseDefault => w.write_varint_u32(4),
        }
    }
}

// ─── RelationRef ─────────────────────────────────────────────────────────────

impl Encode for RelationRef {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.entity.encode(w, b))??;
        b.descend(|b| self.field.encode(w, b))??;
        b.descend(|b| self.on_delete.encode(w, b))??;
        b.descend(|b| self.on_update.encode(w, b))??;
        Ok(())
    }
}

// ─── ComputedKind ────────────────────────────────────────────────────────────

impl Encode for ComputedKind {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            ComputedKind::Materialized => w.write_varint_u32(0),
            ComputedKind::OnDemand => w.write_varint_u32(1),
        }
    }
}

// ─── FieldDef ────────────────────────────────────────────────────────────────

impl Encode for FieldDef {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.data_type.encode(w, b))??;
        b.descend(|b| self.identity.encode(w, b))??;
        b.descend(|b| self.nullable.encode(w, b))??;
        b.descend(|b| self.unique.encode(w, b))??;
        b.descend(|b| self.references.encode(w, b))??;
        b.descend(|b| self.default_expr.encode(w, b))??;
        b.descend(|b| self.check_expr.encode(w, b))??;
        b.descend(|b| self.generated.encode(w, b))??;
        b.descend(|b| self.lookup.encode(w, b))??;
        b.descend(|b| self.auto_assign.encode(w, b))??;
        b.descend(|b| self.collation.encode(w, b))??;
        b.descend(|b| self.comment.encode(w, b))??;
        Ok(())
    }
}

// ─── FieldOp ─────────────────────────────────────────────────────────────────

impl Encode for FieldOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.verb.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.field.encode(w, b))??;
        b.descend(|b| self.def.encode(w, b))??;
        b.descend(|b| self.new_name.encode(w, b))??;
        Ok(())
    }
}

// ─── IndexDirection ──────────────────────────────────────────────────────────

impl Encode for IndexDirection {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            IndexDirection::Ascending => w.write_varint_u32(0),
            IndexDirection::Descending => w.write_varint_u32(1),
        }
    }
}

// ─── IndexKey ────────────────────────────────────────────────────────────────

impl Encode for IndexKey {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            IndexKey::Field { name, direction } => {
                w.write_varint_u32(0)?;
                b.descend(|b| name.encode(w, b))??;
                b.descend(|b| direction.encode(w, b))??;
            }
            IndexKey::Expression { node, direction } => {
                w.write_varint_u32(1)?;
                b.descend(|b| node.encode(w, b))??;
                b.descend(|b| direction.encode(w, b))??;
            }
        }
        Ok(())
    }
}

// ─── IndexMethod ─────────────────────────────────────────────────────────────

impl Encode for IndexMethod {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            IndexMethod::BTree => w.write_varint_u32(0),
            IndexMethod::Hash => w.write_varint_u32(1),
            IndexMethod::Gin => w.write_varint_u32(2),
            IndexMethod::Gist => w.write_varint_u32(3),
            IndexMethod::Vector => w.write_varint_u32(4),
            IndexMethod::Spatial => w.write_varint_u32(5),
            IndexMethod::Custom(sym) => {
                w.write_varint_u32(6)?;
                b.descend(|b| sym.encode(w, b))??;
                Ok(())
            }
        }
    }
}

// ─── IndexOp ─────────────────────────────────────────────────────────────────

impl Encode for IndexOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.verb.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.method.encode(w, b))??;
        b.descend(|b| self.keys.encode(w, b))??;
        b.descend(|b| self.unique.encode(w, b))??;
        b.descend(|b| self.predicate.encode(w, b))??;
        b.descend(|b| self.if_not_exists.encode(w, b))??;
        Ok(())
    }
}

// ─── LookupMethod ────────────────────────────────────────────────────────────

impl Encode for LookupMethod {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            LookupMethod::Hash => w.write_varint_u32(0),
            LookupMethod::Tree => w.write_varint_u32(1),
            LookupMethod::Inverted => w.write_varint_u32(2),
            LookupMethod::Custom(sym) => {
                w.write_varint_u32(3)?;
                b.descend(|b| sym.encode(w, b))??;
                Ok(())
            }
        }
    }
}

// ─── LookupOp ────────────────────────────────────────────────────────────────

impl Encode for LookupOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.verb.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.method.encode(w, b))??;
        b.descend(|b| self.fields.encode(w, b))??;
        b.descend(|b| self.unique.encode(w, b))??;
        b.descend(|b| self.if_not_exists.encode(w, b))??;
        Ok(())
    }
}

// ─── PolicyScope ─────────────────────────────────────────────────────────────

impl Encode for PolicyScope {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            PolicyScope::Read => w.write_varint_u32(0),
            PolicyScope::Write => w.write_varint_u32(1),
            PolicyScope::All => w.write_varint_u32(2),
        }
    }
}

// ─── PolicyOp ────────────────────────────────────────────────────────────────

impl Encode for PolicyOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.verb.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.scope.encode(w, b))??;
        b.descend(|b| self.using_expr.encode(w, b))??;
        b.descend(|b| self.check_expr.encode(w, b))??;
        Ok(())
    }
}

// ─── MaskOp ──────────────────────────────────────────────────────────────────

impl Encode for MaskOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.verb.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.fields.encode(w, b))??;
        b.descend(|b| self.mask_expr.encode(w, b))??;
        Ok(())
    }
}

// ─── QuotaKind ───────────────────────────────────────────────────────────────

impl Encode for QuotaKind {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            QuotaKind::Storage => w.write_varint_u32(0),
            QuotaKind::Count => w.write_varint_u32(1),
            QuotaKind::Rate => w.write_varint_u32(2),
            QuotaKind::Concurrency => w.write_varint_u32(3),
        }
    }
}

// ─── QuotaOp ─────────────────────────────────────────────────────────────────

impl Encode for QuotaOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.verb.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.kind.encode(w, b))??;
        b.descend(|b| self.limit.encode(w, b))??;
        b.descend(|b| self.role.encode(w, b))??;
        Ok(())
    }
}

// ─── AuditEvent ──────────────────────────────────────────────────────────────

impl Encode for AuditEvent {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            AuditEvent::Read => w.write_varint_u32(0),
            AuditEvent::Write => w.write_varint_u32(1),
            AuditEvent::SchemaChange => w.write_varint_u32(2),
            AuditEvent::AccessControl => w.write_varint_u32(3),
            AuditEvent::All => w.write_varint_u32(4),
        }
    }
}

// ─── AuditSink ───────────────────────────────────────────────────────────────

impl Encode for AuditSink {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            AuditSink::Default => w.write_varint_u32(0),
            AuditSink::Named(sym) => {
                w.write_varint_u32(1)?;
                b.descend(|b| sym.encode(w, b))??;
                Ok(())
            }
        }
    }
}

// ─── AuditOp ─────────────────────────────────────────────────────────────────

impl Encode for AuditOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.verb.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.event.encode(w, b))??;
        b.descend(|b| self.sink.encode(w, b))??;
        Ok(())
    }
}

// ─── Grant / Revoke ──────────────────────────────────────────────────────────

impl Encode for Grant {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.privileges.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.roles.encode(w, b))??;
        b.descend(|b| self.with_grant_option.encode(w, b))??;
        Ok(())
    }
}

impl Encode for Revoke {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.privileges.encode(w, b))??;
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.roles.encode(w, b))??;
        b.descend(|b| self.cascade.encode(w, b))??;
        Ok(())
    }
}

// ─── InsertSource ────────────────────────────────────────────────────────────

impl Encode for InsertSource {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            InsertSource::Node(id) => {
                w.write_varint_u32(0)?;
                b.descend(|b| id.encode(w, b))??;
            }
            InsertSource::FromQuery(id) => {
                w.write_varint_u32(1)?;
                b.descend(|b| id.encode(w, b))??;
            }
            InsertSource::Bindings => w.write_varint_u32(2)?,
            InsertSource::FromPath(sym) => {
                w.write_varint_u32(3)?;
                b.descend(|b| sym.encode(w, b))??;
            }
            InsertSource::FromExpr(id) => {
                w.write_varint_u32(4)?;
                b.descend(|b| id.encode(w, b))??;
            }
        }
        Ok(())
    }
}

// ─── DML operations ──────────────────────────────────────────────────────────

impl Encode for Insert {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.source.encode(w, b))??;
        b.descend(|b| self.returning.encode(w, b))??;
        Ok(())
    }
}

impl Encode for Update {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.node.encode(w, b))??;
        Ok(())
    }
}

impl Encode for ReplaceBody {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            ReplaceBody::FromExpr(id) => {
                w.write_varint_u32(0)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ReplaceBody::FromQuery(id) => {
                w.write_varint_u32(1)?;
                b.descend(|b| id.encode(w, b))??;
            }
            ReplaceBody::Bindings => w.write_varint_u32(2)?,
            ReplaceBody::FromPath(sym) => {
                w.write_varint_u32(3)?;
                b.descend(|b| sym.encode(w, b))??;
            }
        }
        Ok(())
    }
}

impl Encode for Replace {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.body.encode(w, b))??;
        b.descend(|b| self.filter.encode(w, b))??;
        Ok(())
    }
}

impl Encode for Delete {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.node.encode(w, b))??;
        Ok(())
    }
}

impl Encode for Upsert {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.node.encode(w, b))??;
        Ok(())
    }
}

impl Encode for Append {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.source.encode(w, b))??;
        b.descend(|b| self.partition_key.encode(w, b))??;
        Ok(())
    }
}

// ─── DQL operations ──────────────────────────────────────────────────────────

impl Encode for Query {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.node.encode(w, b))??;
        Ok(())
    }
}

impl Encode for Probe {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.include_metadata.encode(w, b))??;
        Ok(())
    }
}

impl Encode for DescribeFacet {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            DescribeFacet::Shape => w.write_varint_u32(0),
            DescribeFacet::Fields => w.write_varint_u32(1),
            DescribeFacet::Indexes => w.write_varint_u32(2),
            DescribeFacet::Acl => w.write_varint_u32(3),
            DescribeFacet::Stats => w.write_varint_u32(4),
        }
    }
}

impl Encode for Describe {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.target.encode(w, b))??;
        b.descend(|b| self.facet.encode(w, b))??;
        Ok(())
    }
}

// ─── Tx operations ───────────────────────────────────────────────────────────

impl Encode for IsolationLevel {
    fn encode(&self, w: &mut Writer<'_>, _b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            IsolationLevel::ReadUncommitted => w.write_varint_u32(0),
            IsolationLevel::ReadCommitted => w.write_varint_u32(1),
            IsolationLevel::RepeatableRead => w.write_varint_u32(2),
            IsolationLevel::Snapshot => w.write_varint_u32(3),
            IsolationLevel::Serializable => w.write_varint_u32(4),
        }
    }
}

impl Encode for TxOptions {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.isolation.encode(w, b))??;
        b.descend(|b| self.read_only.encode(w, b))??;
        b.descend(|b| self.label.encode(w, b))??;
        Ok(())
    }
}

impl Encode for TxBegin {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.opts.encode(w, b))??;
        Ok(())
    }
}

impl Encode for TxOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            TxOp::Begin(begin) => {
                w.write_varint_u32(0)?;
                b.descend(|b| begin.encode(w, b))??;
            }
            TxOp::Commit => w.write_varint_u32(1)?,
            TxOp::Rollback => w.write_varint_u32(2)?,
            TxOp::Savepoint(sym) => {
                w.write_varint_u32(3)?;
                b.descend(|b| sym.encode(w, b))??;
            }
            TxOp::ReleaseSavepoint(sym) => {
                w.write_varint_u32(4)?;
                b.descend(|b| sym.encode(w, b))??;
            }
            TxOp::RollbackTo(sym) => {
                w.write_varint_u32(5)?;
                b.descend(|b| sym.encode(w, b))??;
            }
            TxOp::Atomic { ops, opts } => {
                w.write_varint_u32(6)?;
                b.descend(|b| encode_slice(ops.as_slice(), w, b))??;
                b.descend(|b| opts.encode(w, b))??;
            }
        }
        Ok(())
    }
}

// ─── Extension ───────────────────────────────────────────────────────────────

impl Encode for ExtensionId {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.name.encode(w, b))??;
        w.write_varint_u32(self.version)
    }
}

impl Encode for OperationExtension {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.id.encode(w, b))??;
        b.descend(|b| encode_slice(self.payload.as_slice(), w, b))??;
        Ok(())
    }
}

// ─── Operation ───────────────────────────────────────────────────────────────

impl Encode for Operation {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            Operation::Schema(op) => {
                w.write_varint_u32(0)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Field(op) => {
                w.write_varint_u32(1)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Index(op) => {
                w.write_varint_u32(2)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Lookup(op) => {
                w.write_varint_u32(3)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Policy(op) => {
                w.write_varint_u32(4)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Mask(op) => {
                w.write_varint_u32(5)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Quota(op) => {
                w.write_varint_u32(6)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Audit(op) => {
                w.write_varint_u32(7)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Insert(op) => {
                w.write_varint_u32(8)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Update(op) => {
                w.write_varint_u32(9)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Replace(op) => {
                w.write_varint_u32(10)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Delete(op) => {
                w.write_varint_u32(11)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Upsert(op) => {
                w.write_varint_u32(12)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Append(op) => {
                w.write_varint_u32(13)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Query(op) => {
                w.write_varint_u32(14)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Probe(op) => {
                w.write_varint_u32(15)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Describe(op) => {
                w.write_varint_u32(16)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Grant(op) => {
                w.write_varint_u32(17)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Revoke(op) => {
                w.write_varint_u32(18)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Tx(op) => {
                w.write_varint_u32(19)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            Operation::Extension(op) => {
                w.write_varint_u32(20)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
            #[cfg(feature = "raw")]
            Operation::Raw(op) => {
                w.write_varint_u32(21)?;
                b.descend(|b| op.as_ref().encode(w, b))??;
            }
        }
        Ok(())
    }
}

impl Encode for EntityConstraint {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            EntityConstraint::Unique(fields) => {
                w.write_varint_u32(0)?;
                b.descend(|b| encode_slice(fields.as_slice(), w, b))??;
            }
            EntityConstraint::Relation {
                fields,
                ref_entity,
                ref_fields,
                on_delete,
            } => {
                w.write_varint_u32(1)?;
                b.descend(|b| encode_slice(fields.as_slice(), w, b))??;
                b.descend(|b| ref_entity.encode(w, b))??;
                b.descend(|b| encode_slice(ref_fields.as_slice(), w, b))??;
                b.descend(|b| on_delete.encode(w, b))??;
            }
            EntityConstraint::Invariant(expr) => {
                w.write_varint_u32(2)?;
                b.descend(|b| expr.encode(w, b))??;
            }
            EntityConstraint::Identity(fields) => {
                w.write_varint_u32(3)?;
                b.descend(|b| encode_slice(fields.as_slice(), w, b))??;
            }
        }
        Ok(())
    }
}

impl Encode for Field {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.data_type.encode(w, b))??;
        b.descend(|b| self.identity.encode(w, b))??;
        b.descend(|b| self.nullable.encode(w, b))??;
        b.descend(|b| self.has_default.encode(w, b))??;
        b.descend(|b| self.default_expr.encode(w, b))??;
        b.descend(|b| self.unique.encode(w, b))??;
        b.descend(|b| self.references.encode(w, b))??;
        b.descend(|b| self.check.encode(w, b))??;
        b.descend(|b| self.comment.encode(w, b))??;
        b.descend(|b| self.collation.encode(w, b))??;
        b.descend(|b| self.generated.encode(w, b))??;
        b.descend(|b| self.lookup.encode(w, b))??;
        b.descend(|b| self.auto_assign.encode(w, b))??;
        Ok(())
    }
}

impl Encode for Entity {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.namespace.encode(w, b))??;
        b.descend(|b| encode_slice(self.fields.as_slice(), w, b))??;
        b.descend(|b| encode_slice(self.constraints.as_slice(), w, b))??;
        Ok(())
    }
}

// ─── SchemaCatalog types ─────────────────────────────────────────────────────

impl Encode for TypeEntry {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.name.encode(w, b))??;
        b.descend(|b| self.kind.encode(w, b))??;
        b.descend(|b| self.members.encode(w, b))??;
        Ok(())
    }
}

impl Encode for CatalogEntry {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        match self {
            CatalogEntry::Entity(entity) => {
                w.write_varint_u32(0)?;
                b.descend(|b| entity.encode(w, b))??;
            }
            CatalogEntry::Type(type_entry) => {
                w.write_varint_u32(1)?;
                b.descend(|b| type_entry.encode(w, b))??;
            }
            CatalogEntry::Extension { kind, payload } => {
                w.write_varint_u32(2)?;
                b.descend(|b| kind.encode(w, b))??;
                b.descend(|b| encode_slice(payload.as_slice(), w, b))??;
            }
        }
        Ok(())
    }
}

impl Encode for SchemaCatalog {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.id.encode(w, b))??;
        b.descend(|b| encode_slice(self.entries_slice(), w, b))??;
        Ok(())
    }
}

// ─── Program ─────────────────────────────────────────────────────────────────

#[cfg(feature = "raw")]
impl Encode for dol_command::operation::meta::RawOp {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| self.dialect.encode(w, b))??;
        b.descend(|b| self.body.encode(w, b))??;
        b.descend(|b| encode_slice(self.params.as_slice(), w, b))??;
        Ok(())
    }
}

impl Encode for Program {
    fn encode(&self, w: &mut Writer<'_>, b: &mut Budget) -> Result<(), EncodeError> {
        b.descend(|b| encode_slice(self.operations.as_slice(), w, b))??;
        b.descend(|b| self.arena.encode(w, b))??;
        b.descend(|b| self.interner.encode(w, b))??;
        b.descend(|b| self.schema_catalog.encode(w, b))??;
        Ok(())
    }
}
