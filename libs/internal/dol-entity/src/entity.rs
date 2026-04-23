use crate::constraint::EntityConstraint;
use crate::field::Field;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Entity {
    pub name:        &'static str,
    pub namespace:   Option<&'static str>,
    pub fields:      Vec<Field>,
    pub constraints: Vec<EntityConstraint>,
}

impl Entity {
    pub fn new(name: &'static str, fields: Vec<Field>) -> Self {
        Self { name, namespace: None, fields, constraints: Vec::new() }
    }

    pub fn with_namespace(mut self, ns: &'static str) -> Self {
        self.namespace = Some(ns);
        self
    }

    pub fn with_constraints(mut self, constraints: Vec<EntityConstraint>) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn qualified_name(&self) -> String {
        match self.namespace {
            Some(ns) => format!("{}.{}", ns, self.name),
            None     => self.name.to_string(),
        }
    }

    pub fn field(&self, name: &str) -> &Field {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("field '{}' not found in entity '{}'", name, self.name))
    }

    pub fn try_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn field_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.fields.iter().map(|f| f.name)
    }

    pub fn primary_keys(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|f| f.primary_key)
    }

    pub fn non_pk_fields(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|f| !f.primary_key)
    }

    pub fn field_list(&self) -> String {
        self.fields.iter().map(|f| f.name).collect::<Vec<_>>().join(", ")
    }
}
