use dol_expr::types::DataType;

use crate::constraint::{FkAction, ForeignKeyRef, GeneratedKind};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Field {
    pub name:          &'static str,
    pub data_type:     DataType,
    pub primary_key:   bool,
    pub nullable:      bool,
    pub has_default:   bool,
    pub default_expr:  Option<&'static str>,
    pub unique:        bool,
    pub references:    Option<ForeignKeyRef>,
    pub check:         Option<&'static str>,
    pub comment:       Option<&'static str>,
    pub collation:     Option<&'static str>,
    pub generated:     Option<(GeneratedKind, &'static str)>,
    pub indexed:       bool,
    pub auto_increment: bool,
}

impl Field {
    pub fn new(name: &'static str, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            primary_key:   false,
            nullable:      false,
            has_default:   false,
            default_expr:  None,
            unique:        false,
            references:    None,
            check:         None,
            comment:       None,
            collation:     None,
            generated:     None,
            indexed:       false,
            auto_increment: false,
        }
    }

    pub fn primary_key(mut self) -> Self       { self.primary_key = true; self }
    pub fn nullable(mut self) -> Self          { self.nullable = true; self }
    pub fn optional(mut self) -> Self          { self.nullable = true; self }
    pub fn required(self) -> Self              { self }
    pub fn unique(mut self) -> Self            { self.unique = true; self }
    pub fn auto_increment(mut self) -> Self    { self.auto_increment = true; self }
    pub fn indexed(mut self) -> Self           { self.indexed = true; self }

    pub fn default(mut self, expr: &'static str) -> Self {
        self.has_default  = true;
        self.default_expr = Some(expr);
        self
    }

    pub fn comment(mut self, c: &'static str) -> Self {
        self.comment = Some(c);
        self
    }

    pub fn collation(mut self, c: &'static str) -> Self {
        self.collation = Some(c);
        self
    }

    pub fn check(mut self, expr: &'static str) -> Self {
        self.check = Some(expr);
        self
    }

    pub fn references(
        mut self,
        table:     &'static str,
        column:    &'static str,
        on_delete: FkAction,
        on_update: FkAction,
    ) -> Self {
        self.references = Some(ForeignKeyRef { table, column, on_delete, on_update });
        self
    }

    pub fn generated_stored(mut self, expr: &'static str) -> Self {
        self.generated = Some((GeneratedKind::Stored, expr));
        self
    }

    pub fn generated_virtual(mut self, expr: &'static str) -> Self {
        self.generated = Some((GeneratedKind::Virtual, expr));
        self
    }
}
