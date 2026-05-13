//! Field definition.
//!
//! A [`Field`] is a named, typed member of an [`Entity`]. [`FieldType`]
//! distinguishes scalar columns, relation pointers, and computed
//! expressions.
//!
//! [`Entity`]: crate::schema::entity::Entity

extern crate alloc;

use dol_cas::handle::{FieldId, NodeId, RelationId, StrId};
use dol_core::data_type::DataType;
use dol_core::literal::Literal;

/// A field (column / attribute / property) of an entity.
///
/// Per `dol-rewrite-plan-v2.md` §8.6 line 1438.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Arena-local identifier for this field.
    pub id: FieldId,
    /// Interned name. Resolved via `SchemaCatalog.strings`.
    pub name: StrId,
    /// Type category: scalar, relation, or computed.
    pub ty: FieldType,
    /// Whether the field accepts NULL values.
    pub nullable: bool,
    /// Optional default value.
    pub default: Option<Literal<'static>>,
}

impl Field {
    /// Construct a new scalar field with no default.
    #[must_use]
    pub fn new_scalar(id: FieldId, name: StrId, data_type: DataType, nullable: bool) -> Self {
        Self {
            id,
            name,
            ty: FieldType::Scalar(data_type),
            nullable,
            default: None,
        }
    }
}

/// The type category of a field.
///
/// Per `dol-rewrite-plan-v2.md` §8.6 line 1446.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FieldType {
    /// A primitive / composite data type.
    Scalar(DataType),
    /// A pointer to a relation (foreign key).
    Relation(RelationId),
    /// A derived / virtual column whose value is an expression.
    Computed {
        /// The expression node producing this field's value.
        expr: NodeId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use dol_core::ids::Id;

    fn make_field_id(n: u32) -> FieldId {
        Id::from_u32(n).unwrap()
    }

    fn make_str_id(n: u32) -> StrId {
        Id::from_u32(n).unwrap()
    }

    #[test]
    fn field_new_scalar_basic() {
        let f = Field::new_scalar(
            make_field_id(1),
            make_str_id(10),
            DataType::Int64,
            false,
        );
        assert_eq!(f.id, make_field_id(1));
        assert_eq!(f.name, make_str_id(10));
        assert!(!f.nullable);
        assert!(f.default.is_none());
        assert!(matches!(f.ty, FieldType::Scalar(DataType::Int64)));
    }

    #[test]
    fn field_debug_round_trip() {
        let f = Field::new_scalar(
            make_field_id(2),
            make_str_id(20),
            DataType::Bool,
            true,
        );
        let dbg = format!("{f:?}");
        assert!(dbg.contains("Field"));
        assert!(dbg.contains("nullable: true"));
    }

    #[test]
    fn field_type_computed_variant() {
        let node_id: NodeId = Id::from_u32(99).unwrap();
        let ty = FieldType::Computed { expr: node_id };
        let FieldType::Computed { expr } = ty else {
            panic!("expected Computed");
        };
        assert_eq!(expr, node_id);
    }
}
