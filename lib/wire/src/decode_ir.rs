//! [`Decode`] impls for `dol-ir`, `dol-schema`, and the top-level [`Program`].

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use dol_core::policy::Budget;
use dol_expr::ids::NodeId;
use dol_ir::operation::acl::{
    AuditEvent, AuditOp, AuditSink, Grant, MaskOp, PolicyOp, PolicyScope, QuotaKind, QuotaOp,
    Revoke,
};
use dol_ir::operation::ddl::{
    FieldDef, FieldOp, IndexDirection, IndexKey, IndexMethod, IndexOp, LookupMethod, LookupOp,
    SchemaBody, SchemaOp, TypeBody,
};
use dol_ir::operation::dml::{
    Append, Delete, Insert, InsertSource, Replace, ReplaceBody, Update, Upsert,
};
use dol_ir::operation::dql::{Describe, DescribeFacet, Probe, Query};
use dol_ir::operation::meta::{ExtensionId, OperationExtension};
use dol_ir::operation::tx::{IsolationLevel, TxBegin, TxOp, TxOptions};
use dol_ir::operation::{Operation, StructuralVerb};
use dol_ir::privilege::Privilege;
use dol_ir::schema_catalog::{CatalogEntry, SchemaCatalog, TypeEntry};
use dol_ir::schema_ref::{CatalogId, SchemaId, SchemaRef};
use dol_ir::target::{Locator, SchemaBinding, Symbol, Target, TargetKind};
use dol_ir::Program;
use dol_schema::constraint::{ComputedKind, EntityConstraint, RefAction, RelationRef};
use dol_schema::{Entity, Field};

use crate::decoder::{Decode, DecodeError, Reader};

// ─── Symbol ──────────────────────────────────────────────────────────────────

impl Decode for Symbol {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let id = budget.descend(|b| dol_expr::ids::StrId::decode(reader, b))??;
        Ok(Symbol::new(id))
    }
}

// ─── CatalogId / SchemaId / SchemaRef ────────────────────────────────────────

impl Decode for CatalogId {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        Ok(CatalogId(reader.read_varint_u32()?))
    }
}

impl Decode for SchemaId {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        Ok(SchemaId(reader.read_varint_u32()?))
    }
}

impl Decode for SchemaRef {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let catalog = budget.descend(|b| CatalogId::decode(reader, b))??;
        let schema = budget.descend(|b| SchemaId::decode(reader, b))??;
        Ok(SchemaRef { catalog, schema })
    }
}

// ─── Locator ─────────────────────────────────────────────────────────────────

impl Decode for Locator {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let namespace = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let path = budget.descend(|b| smallvec::SmallVec::<[Symbol; 2]>::decode(reader, b))??;
        Ok(Locator { namespace, name, path })
    }
}

// ─── TargetKind ──────────────────────────────────────────────────────────────

impl Decode for TargetKind {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(TargetKind::Relation),
            1 => Ok(TargetKind::Document),
            2 => Ok(TargetKind::KeyValue),
            3 => Ok(TargetKind::Blob),
            4 => Ok(TargetKind::FileTree),
            5 => Ok(TargetKind::ApiResource),
            6 => Ok(TargetKind::StreamTopic),
            7 => Ok(TargetKind::Virtual),
            8 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(TargetKind::Custom(sym))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "TargetKind", seen }),
        }
    }
}

// ─── SchemaBinding ───────────────────────────────────────────────────────────

impl Decode for SchemaBinding {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let schema_ref = budget.descend(|b| SchemaRef::decode(reader, b))??;
                Ok(SchemaBinding::Declared(schema_ref))
            }
            1 => Ok(SchemaBinding::Inferred),
            2 => Ok(SchemaBinding::Opaque),
            seen => Err(DecodeError::InvalidVariant { type_name: "SchemaBinding", seen }),
        }
    }
}

// ─── Target ──────────────────────────────────────────────────────────────────

impl Decode for Target {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let kind = budget.descend(|b| TargetKind::decode(reader, b))??;
        let locator = budget.descend(|b| Locator::decode(reader, b))??;
        let alias = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        let schema = budget.descend(|b| SchemaBinding::decode(reader, b))??;
        Ok(Target { kind, locator, alias, schema })
    }
}

// ─── Privilege ───────────────────────────────────────────────────────────────

impl Decode for Privilege {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(Privilege::Select),
            1 => Ok(Privilege::Insert),
            2 => Ok(Privilege::Update),
            3 => Ok(Privilege::Delete),
            4 => Ok(Privilege::All),
            5 => Ok(Privilege::Usage),
            6 => Ok(Privilege::Create),
            7 => Ok(Privilege::Connect),
            8 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(Privilege::Custom(sym))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "Privilege", seen }),
        }
    }
}

// ─── StructuralVerb ──────────────────────────────────────────────────────────

impl Decode for StructuralVerb {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(StructuralVerb::Create),
            1 => Ok(StructuralVerb::Drop),
            2 => Ok(StructuralVerb::Alter),
            3 => Ok(StructuralVerb::Rename),
            4 => Ok(StructuralVerb::Truncate),
            seen => Err(DecodeError::InvalidVariant { type_name: "StructuralVerb", seen }),
        }
    }
}

// ─── TypeBody ────────────────────────────────────────────────────────────────

impl Decode for TypeBody {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(TypeBody::Enum),
            1 => Ok(TypeBody::Composite),
            2 => Ok(TypeBody::Distinct),
            3 => Ok(TypeBody::Other),
            seen => Err(DecodeError::InvalidVariant { type_name: "TypeBody", seen }),
        }
    }
}

// ─── SchemaBody ──────────────────────────────────────────────────────────────

impl Decode for SchemaBody {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let schema = budget.descend(|b| SchemaRef::decode(reader, b))??;
                let if_not_exists = budget.descend(|b| bool::decode(reader, b))??;
                Ok(SchemaBody::Entity { schema, if_not_exists })
            }
            1 => {
                let schema = budget.descend(|b| SchemaRef::decode(reader, b))??;
                let body = budget.descend(|b| TypeBody::decode(reader, b))??;
                Ok(SchemaBody::Type { schema, body })
            }
            2 => Ok(SchemaBody::Reference),
            seen => Err(DecodeError::InvalidVariant { type_name: "SchemaBody", seen }),
        }
    }
}

// ─── SchemaOp ────────────────────────────────────────────────────────────────

impl Decode for SchemaOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let verb = budget.descend(|b| StructuralVerb::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let body = budget.descend(|b| SchemaBody::decode(reader, b))??;
        let new_name = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        Ok(SchemaOp { verb, target, body, new_name })
    }
}

// ─── RefAction ───────────────────────────────────────────────────────────────

impl Decode for RefAction {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(RefAction::Forbid),
            1 => Ok(RefAction::Cascade),
            2 => Ok(RefAction::Detach),
            3 => Ok(RefAction::Reject),
            4 => Ok(RefAction::UseDefault),
            seen => Err(DecodeError::InvalidVariant { type_name: "RefAction", seen }),
        }
    }
}

// ─── RelationRef ─────────────────────────────────────────────────────────────

impl Decode for RelationRef {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let entity = budget.descend(|b| Arc::<str>::decode(reader, b))??;
        let field = budget.descend(|b| Arc::<str>::decode(reader, b))??;
        let on_delete = budget.descend(|b| RefAction::decode(reader, b))??;
        let on_update = budget.descend(|b| RefAction::decode(reader, b))??;
        Ok(RelationRef { entity, field, on_delete, on_update })
    }
}

// ─── ComputedKind ────────────────────────────────────────────────────────────

impl Decode for ComputedKind {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(ComputedKind::Materialized),
            1 => Ok(ComputedKind::OnDemand),
            seen => Err(DecodeError::InvalidVariant { type_name: "ComputedKind", seen }),
        }
    }
}

// ─── FieldDef ────────────────────────────────────────────────────────────────

impl Decode for FieldDef {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let data_type = budget.descend(|b| dol_core::DataType::decode(reader, b))??;
        let identity = budget.descend(|b| bool::decode(reader, b))??;
        let nullable = budget.descend(|b| bool::decode(reader, b))??;
        let unique = budget.descend(|b| bool::decode(reader, b))??;
        let references = budget.descend(|b| Option::<RelationRef>::decode(reader, b))??;
        let default_expr = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        let check_expr = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        let generated = budget.descend(|b| Option::<(ComputedKind, NodeId)>::decode(reader, b))??;
        let lookup = budget.descend(|b| bool::decode(reader, b))??;
        let auto_assign = budget.descend(|b| bool::decode(reader, b))??;
        let collation = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        let comment = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        Ok(FieldDef {
            name, data_type, identity, nullable, unique, references, default_expr, check_expr,
            generated, lookup, auto_assign, collation, comment,
        })
    }
}

// ─── FieldOp ─────────────────────────────────────────────────────────────────

impl Decode for FieldOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let verb = budget.descend(|b| StructuralVerb::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let field = budget.descend(|b| Symbol::decode(reader, b))??;
        let def = budget.descend(|b| Option::<FieldDef>::decode(reader, b))??;
        let new_name = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        Ok(FieldOp { verb, target, field, def, new_name })
    }
}

// ─── IndexDirection ──────────────────────────────────────────────────────────

impl Decode for IndexDirection {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(IndexDirection::Ascending),
            1 => Ok(IndexDirection::Descending),
            seen => Err(DecodeError::InvalidVariant { type_name: "IndexDirection", seen }),
        }
    }
}

// ─── IndexKey ────────────────────────────────────────────────────────────────

impl Decode for IndexKey {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let name = budget.descend(|b| Symbol::decode(reader, b))??;
                let direction = budget.descend(|b| IndexDirection::decode(reader, b))??;
                Ok(IndexKey::Field { name, direction })
            }
            1 => {
                let node = budget.descend(|b| NodeId::decode(reader, b))??;
                let direction = budget.descend(|b| IndexDirection::decode(reader, b))??;
                Ok(IndexKey::Expression { node, direction })
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "IndexKey", seen }),
        }
    }
}

// ─── IndexMethod ─────────────────────────────────────────────────────────────

impl Decode for IndexMethod {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(IndexMethod::BTree),
            1 => Ok(IndexMethod::Hash),
            2 => Ok(IndexMethod::Gin),
            3 => Ok(IndexMethod::Gist),
            4 => Ok(IndexMethod::Vector),
            5 => Ok(IndexMethod::Spatial),
            6 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(IndexMethod::Custom(sym))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "IndexMethod", seen }),
        }
    }
}

// ─── IndexOp ─────────────────────────────────────────────────────────────────

impl Decode for IndexOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let verb = budget.descend(|b| StructuralVerb::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let method = budget.descend(|b| IndexMethod::decode(reader, b))??;
        let keys = budget.descend(|b| smallvec::SmallVec::<[IndexKey; 2]>::decode(reader, b))??;
        let unique = budget.descend(|b| bool::decode(reader, b))??;
        let predicate = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        let if_not_exists = budget.descend(|b| bool::decode(reader, b))??;
        Ok(IndexOp { verb, target, name, method, keys, unique, predicate, if_not_exists })
    }
}

// ─── LookupMethod ────────────────────────────────────────────────────────────

impl Decode for LookupMethod {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(LookupMethod::Hash),
            1 => Ok(LookupMethod::Tree),
            2 => Ok(LookupMethod::Inverted),
            3 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(LookupMethod::Custom(sym))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "LookupMethod", seen }),
        }
    }
}

// ─── LookupOp ────────────────────────────────────────────────────────────────

impl Decode for LookupOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let verb = budget.descend(|b| StructuralVerb::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let method = budget.descend(|b| LookupMethod::decode(reader, b))??;
        let fields = budget.descend(|b| smallvec::SmallVec::<[Symbol; 2]>::decode(reader, b))??;
        let unique = budget.descend(|b| bool::decode(reader, b))??;
        let if_not_exists = budget.descend(|b| bool::decode(reader, b))??;
        Ok(LookupOp { verb, target, name, method, fields, unique, if_not_exists })
    }
}

// ─── PolicyScope ─────────────────────────────────────────────────────────────

impl Decode for PolicyScope {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(PolicyScope::Read),
            1 => Ok(PolicyScope::Write),
            2 => Ok(PolicyScope::All),
            seen => Err(DecodeError::InvalidVariant { type_name: "PolicyScope", seen }),
        }
    }
}

// ─── PolicyOp ────────────────────────────────────────────────────────────────

impl Decode for PolicyOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let verb = budget.descend(|b| StructuralVerb::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let scope = budget.descend(|b| PolicyScope::decode(reader, b))??;
        let using_expr = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        let check_expr = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        Ok(PolicyOp { verb, target, name, scope, using_expr, check_expr })
    }
}

// ─── MaskOp ──────────────────────────────────────────────────────────────────

impl Decode for MaskOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let verb = budget.descend(|b| StructuralVerb::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let fields = budget.descend(|b| smallvec::SmallVec::<[Symbol; 2]>::decode(reader, b))??;
        let mask_expr = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        Ok(MaskOp { verb, target, name, fields, mask_expr })
    }
}

// ─── QuotaKind ───────────────────────────────────────────────────────────────

impl Decode for QuotaKind {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(QuotaKind::Storage),
            1 => Ok(QuotaKind::Count),
            2 => Ok(QuotaKind::Rate),
            3 => Ok(QuotaKind::Concurrency),
            seen => Err(DecodeError::InvalidVariant { type_name: "QuotaKind", seen }),
        }
    }
}

// ─── QuotaOp ─────────────────────────────────────────────────────────────────

impl Decode for QuotaOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let verb = budget.descend(|b| StructuralVerb::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let kind = budget.descend(|b| QuotaKind::decode(reader, b))??;
        let limit = budget.descend(|b| u64::decode(reader, b))??;
        let role = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        Ok(QuotaOp { verb, target, name, kind, limit, role })
    }
}

// ─── AuditEvent ──────────────────────────────────────────────────────────────

impl Decode for AuditEvent {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(AuditEvent::Read),
            1 => Ok(AuditEvent::Write),
            2 => Ok(AuditEvent::SchemaChange),
            3 => Ok(AuditEvent::AccessControl),
            4 => Ok(AuditEvent::All),
            seen => Err(DecodeError::InvalidVariant { type_name: "AuditEvent", seen }),
        }
    }
}

// ─── AuditSink ───────────────────────────────────────────────────────────────

impl Decode for AuditSink {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(AuditSink::Default),
            1 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(AuditSink::Named(sym))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "AuditSink", seen }),
        }
    }
}

// ─── AuditOp ─────────────────────────────────────────────────────────────────

impl Decode for AuditOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let verb = budget.descend(|b| StructuralVerb::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let event = budget.descend(|b| AuditEvent::decode(reader, b))??;
        let sink = budget.descend(|b| AuditSink::decode(reader, b))??;
        Ok(AuditOp { verb, target, name, event, sink })
    }
}

// ─── Grant / Revoke ──────────────────────────────────────────────────────────

impl Decode for Grant {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let privileges = budget.descend(|b| smallvec::SmallVec::<[Privilege; 2]>::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let roles = budget.descend(|b| smallvec::SmallVec::<[Symbol; 1]>::decode(reader, b))??;
        let with_grant_option = budget.descend(|b| bool::decode(reader, b))??;
        Ok(Grant { privileges, target, roles, with_grant_option })
    }
}

impl Decode for Revoke {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let privileges = budget.descend(|b| smallvec::SmallVec::<[Privilege; 2]>::decode(reader, b))??;
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let roles = budget.descend(|b| smallvec::SmallVec::<[Symbol; 1]>::decode(reader, b))??;
        let cascade = budget.descend(|b| bool::decode(reader, b))??;
        Ok(Revoke { privileges, target, roles, cascade })
    }
}

// ─── InsertSource ────────────────────────────────────────────────────────────

impl Decode for InsertSource {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let id = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(InsertSource::Node(id))
            }
            1 => {
                let id = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(InsertSource::FromQuery(id))
            }
            2 => Ok(InsertSource::Bindings),
            3 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(InsertSource::FromPath(sym))
            }
            4 => {
                let id = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(InsertSource::FromExpr(id))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "InsertSource", seen }),
        }
    }
}

// ─── DML operations ──────────────────────────────────────────────────────────

impl Decode for Insert {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let source = budget.descend(|b| InsertSource::decode(reader, b))??;
        let returning = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        Ok(Insert { target, source, returning })
    }
}

impl Decode for Update {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let node = budget.descend(|b| NodeId::decode(reader, b))??;
        Ok(Update { target, node })
    }
}

impl Decode for ReplaceBody {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let id = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(ReplaceBody::FromExpr(id))
            }
            1 => {
                let id = budget.descend(|b| NodeId::decode(reader, b))??;
                Ok(ReplaceBody::FromQuery(id))
            }
            2 => Ok(ReplaceBody::Bindings),
            3 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(ReplaceBody::FromPath(sym))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "ReplaceBody", seen }),
        }
    }
}

impl Decode for Replace {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let body = budget.descend(|b| ReplaceBody::decode(reader, b))??;
        let filter = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        Ok(Replace { target, body, filter })
    }
}

impl Decode for Delete {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let node = budget.descend(|b| NodeId::decode(reader, b))??;
        Ok(Delete { target, node })
    }
}

impl Decode for Upsert {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let node = budget.descend(|b| NodeId::decode(reader, b))??;
        Ok(Upsert { target, node })
    }
}

impl Decode for Append {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let source = budget.descend(|b| InsertSource::decode(reader, b))??;
        let partition_key = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        Ok(Append { target, source, partition_key })
    }
}

// ─── DQL operations ──────────────────────────────────────────────────────────

impl Decode for Query {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let node = budget.descend(|b| Option::<NodeId>::decode(reader, b))??;
        Ok(Query { target, node })
    }
}

impl Decode for Probe {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let include_metadata = budget.descend(|b| bool::decode(reader, b))??;
        Ok(Probe { target, include_metadata })
    }
}

impl Decode for DescribeFacet {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(DescribeFacet::Shape),
            1 => Ok(DescribeFacet::Fields),
            2 => Ok(DescribeFacet::Indexes),
            3 => Ok(DescribeFacet::Acl),
            4 => Ok(DescribeFacet::Stats),
            seen => Err(DecodeError::InvalidVariant { type_name: "DescribeFacet", seen }),
        }
    }
}

impl Decode for Describe {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let target = budget.descend(|b| Target::decode(reader, b))??;
        let facet = budget.descend(|b| DescribeFacet::decode(reader, b))??;
        Ok(Describe { target, facet })
    }
}

// ─── Tx operations ───────────────────────────────────────────────────────────

impl Decode for IsolationLevel {
    fn decode(reader: &mut Reader<'_>, _b: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => Ok(IsolationLevel::ReadUncommitted),
            1 => Ok(IsolationLevel::ReadCommitted),
            2 => Ok(IsolationLevel::RepeatableRead),
            3 => Ok(IsolationLevel::Snapshot),
            4 => Ok(IsolationLevel::Serializable),
            seen => Err(DecodeError::InvalidVariant { type_name: "IsolationLevel", seen }),
        }
    }
}

impl Decode for TxOptions {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let isolation = budget.descend(|b| Option::<IsolationLevel>::decode(reader, b))??;
        let read_only = budget.descend(|b| bool::decode(reader, b))??;
        let label = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        Ok(TxOptions { isolation, read_only, label })
    }
}

impl Decode for TxBegin {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let opts = budget.descend(|b| TxOptions::decode(reader, b))??;
        Ok(TxBegin { opts })
    }
}

impl Decode for TxOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let begin = budget.descend(|b| TxBegin::decode(reader, b))??;
                Ok(TxOp::Begin(begin))
            }
            1 => Ok(TxOp::Commit),
            2 => Ok(TxOp::Rollback),
            3 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(TxOp::Savepoint(sym))
            }
            4 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(TxOp::ReleaseSavepoint(sym))
            }
            5 => {
                let sym = budget.descend(|b| Symbol::decode(reader, b))??;
                Ok(TxOp::RollbackTo(sym))
            }
            6 => {
                let ops = budget.descend(|b| Vec::<Operation>::decode(reader, b))??;
                let opts = budget.descend(|b| TxOptions::decode(reader, b))??;
                Ok(TxOp::Atomic { ops, opts })
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "TxOp", seen }),
        }
    }
}

// ─── Extension ───────────────────────────────────────────────────────────────

impl Decode for ExtensionId {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let version = reader.read_varint_u32()?;
        Ok(ExtensionId { name, version })
    }
}

impl Decode for OperationExtension {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let id = budget.descend(|b| ExtensionId::decode(reader, b))??;
        let payload = budget.descend(|b| Vec::<u8>::decode(reader, b))??;
        Ok(OperationExtension { id, payload })
    }
}

// ─── Operation ───────────────────────────────────────────────────────────────

impl Decode for Operation {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let op = budget.descend(|b| SchemaOp::decode(reader, b))??;
                Ok(Operation::Schema(Box::new(op)))
            }
            1 => {
                let op = budget.descend(|b| FieldOp::decode(reader, b))??;
                Ok(Operation::Field(Box::new(op)))
            }
            2 => {
                let op = budget.descend(|b| IndexOp::decode(reader, b))??;
                Ok(Operation::Index(Box::new(op)))
            }
            3 => {
                let op = budget.descend(|b| LookupOp::decode(reader, b))??;
                Ok(Operation::Lookup(Box::new(op)))
            }
            4 => {
                let op = budget.descend(|b| PolicyOp::decode(reader, b))??;
                Ok(Operation::Policy(Box::new(op)))
            }
            5 => {
                let op = budget.descend(|b| MaskOp::decode(reader, b))??;
                Ok(Operation::Mask(Box::new(op)))
            }
            6 => {
                let op = budget.descend(|b| QuotaOp::decode(reader, b))??;
                Ok(Operation::Quota(Box::new(op)))
            }
            7 => {
                let op = budget.descend(|b| AuditOp::decode(reader, b))??;
                Ok(Operation::Audit(Box::new(op)))
            }
            8 => {
                let op = budget.descend(|b| Insert::decode(reader, b))??;
                Ok(Operation::Insert(Box::new(op)))
            }
            9 => {
                let op = budget.descend(|b| Update::decode(reader, b))??;
                Ok(Operation::Update(Box::new(op)))
            }
            10 => {
                let op = budget.descend(|b| Replace::decode(reader, b))??;
                Ok(Operation::Replace(Box::new(op)))
            }
            11 => {
                let op = budget.descend(|b| Delete::decode(reader, b))??;
                Ok(Operation::Delete(Box::new(op)))
            }
            12 => {
                let op = budget.descend(|b| Upsert::decode(reader, b))??;
                Ok(Operation::Upsert(Box::new(op)))
            }
            13 => {
                let op = budget.descend(|b| Append::decode(reader, b))??;
                Ok(Operation::Append(Box::new(op)))
            }
            14 => {
                let op = budget.descend(|b| Query::decode(reader, b))??;
                Ok(Operation::Query(Box::new(op)))
            }
            15 => {
                let op = budget.descend(|b| Probe::decode(reader, b))??;
                Ok(Operation::Probe(Box::new(op)))
            }
            16 => {
                let op = budget.descend(|b| Describe::decode(reader, b))??;
                Ok(Operation::Describe(Box::new(op)))
            }
            17 => {
                let op = budget.descend(|b| Grant::decode(reader, b))??;
                Ok(Operation::Grant(Box::new(op)))
            }
            18 => {
                let op = budget.descend(|b| Revoke::decode(reader, b))??;
                Ok(Operation::Revoke(Box::new(op)))
            }
            19 => {
                let op = budget.descend(|b| TxOp::decode(reader, b))??;
                Ok(Operation::Tx(Box::new(op)))
            }
            20 => {
                let op = budget.descend(|b| OperationExtension::decode(reader, b))??;
                Ok(Operation::Extension(Box::new(op)))
            }
            #[cfg(feature = "raw")]
            21 => {
                let op = budget.descend(|b| dol_ir::operation::meta::RawOp::decode(reader, b))??;
                Ok(Operation::Raw(Box::new(op)))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "Operation", seen }),
        }
    }
}

// ─── RawOp ───────────────────────────────────────────────────────────────────

#[cfg(feature = "raw")]
impl Decode for dol_ir::operation::meta::RawOp {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let dialect = budget.descend(|b| Option::<Symbol>::decode(reader, b))??;
        let body = budget.descend(|b| String::decode(reader, b))??;
        let params = budget.descend(|b| smallvec::SmallVec::<[NodeId; 4]>::decode(reader, b))??;
        Ok(dol_ir::operation::meta::RawOp { dialect, body, params })
    }
}

// ─── dol-schema types ────────────────────────────────────────────────────────

impl Decode for EntityConstraint {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let fields = budget.descend(|b| Vec::<Arc<str>>::decode(reader, b))??;
                Ok(EntityConstraint::Unique(fields))
            }
            1 => {
                let fields = budget.descend(|b| Vec::<Arc<str>>::decode(reader, b))??;
                let ref_entity = budget.descend(|b| Arc::<str>::decode(reader, b))??;
                let ref_fields = budget.descend(|b| Vec::<Arc<str>>::decode(reader, b))??;
                let on_delete = budget.descend(|b| RefAction::decode(reader, b))??;
                Ok(EntityConstraint::Relation { fields, ref_entity, ref_fields, on_delete })
            }
            2 => {
                let expr = budget.descend(|b| Arc::<str>::decode(reader, b))??;
                Ok(EntityConstraint::Invariant(expr))
            }
            3 => {
                let fields = budget.descend(|b| Vec::<Arc<str>>::decode(reader, b))??;
                Ok(EntityConstraint::Identity(fields))
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "EntityConstraint", seen }),
        }
    }
}

impl Decode for Field {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let name = budget.descend(|b| Arc::<str>::decode(reader, b))??;
        let data_type = budget.descend(|b| dol_core::DataType::decode(reader, b))??;
        let identity = budget.descend(|b| bool::decode(reader, b))??;
        let nullable = budget.descend(|b| bool::decode(reader, b))??;
        let has_default = budget.descend(|b| bool::decode(reader, b))??;
        let default_expr = budget.descend(|b| Option::<Arc<str>>::decode(reader, b))??;
        let unique = budget.descend(|b| bool::decode(reader, b))??;
        let references = budget.descend(|b| Option::<RelationRef>::decode(reader, b))??;
        let check = budget.descend(|b| Option::<Arc<str>>::decode(reader, b))??;
        let comment = budget.descend(|b| Option::<Arc<str>>::decode(reader, b))??;
        let collation = budget.descend(|b| Option::<Arc<str>>::decode(reader, b))??;
        let generated = budget.descend(|b| Option::<(ComputedKind, Arc<str>)>::decode(reader, b))??;
        let lookup = budget.descend(|b| bool::decode(reader, b))??;
        let auto_assign = budget.descend(|b| bool::decode(reader, b))??;
        Ok(Field {
            name, data_type, identity, nullable, has_default, default_expr, unique, references,
            check, comment, collation, generated, lookup, auto_assign,
        })
    }
}

impl Decode for Entity {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let name = budget.descend(|b| Arc::<str>::decode(reader, b))??;
        let namespace = budget.descend(|b| Option::<Arc<str>>::decode(reader, b))??;
        let fields = budget.descend(|b| Vec::<Field>::decode(reader, b))??;
        let constraints = budget.descend(|b| Vec::<EntityConstraint>::decode(reader, b))??;
        Ok(Entity { name, namespace, fields, constraints })
    }
}

// ─── SchemaCatalog types ─────────────────────────────────────────────────────

impl Decode for TypeEntry {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let name = budget.descend(|b| Symbol::decode(reader, b))??;
        let kind = budget.descend(|b| TypeBody::decode(reader, b))??;
        let members = budget.descend(|b| smallvec::SmallVec::<[Symbol; 4]>::decode(reader, b))??;
        Ok(TypeEntry { name, kind, members })
    }
}

impl Decode for CatalogEntry {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        match reader.read_varint_u32()? {
            0 => {
                let entity = budget.descend(|b| Entity::decode(reader, b))??;
                Ok(CatalogEntry::Entity(entity))
            }
            1 => {
                let type_entry = budget.descend(|b| TypeEntry::decode(reader, b))??;
                Ok(CatalogEntry::Type(type_entry))
            }
            2 => {
                let kind = budget.descend(|b| Symbol::decode(reader, b))??;
                let payload = budget.descend(|b| Vec::<u8>::decode(reader, b))??;
                Ok(CatalogEntry::Extension { kind, payload })
            }
            seen => Err(DecodeError::InvalidVariant { type_name: "CatalogEntry", seen }),
        }
    }
}

impl Decode for SchemaCatalog {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let id = budget.descend(|b| CatalogId::decode(reader, b))??;
        let entries = budget.descend(|b| Vec::<CatalogEntry>::decode(reader, b))??;
        let mut catalog = SchemaCatalog::with_id(id);
        for entry in entries {
            catalog.insert(entry);
        }
        Ok(catalog)
    }
}

// ─── Program ─────────────────────────────────────────────────────────────────

impl Decode for Program {
    fn decode(reader: &mut Reader<'_>, budget: &mut Budget) -> Result<Self, DecodeError> {
        let operations = budget.descend(|b| Vec::<Operation>::decode(reader, b))??;
        let arena = budget.descend(|b| dol_expr::ExprArena::decode(reader, b))??;
        let interner = budget.descend(|b| dol_expr::Interner::decode(reader, b))??;
        let schema_catalog = budget.descend(|b| Option::<SchemaCatalog>::decode(reader, b))??;
        Ok(Program { operations, arena, interner, schema_catalog })
    }
}
