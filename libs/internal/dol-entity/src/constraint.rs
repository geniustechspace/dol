use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FkAction {
    NoAction,
    Cascade,
    SetNull,
    Restrict,
    SetDefault,
}

impl fmt::Display for FkAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAction   => write!(f, "NO ACTION"),
            Self::Cascade    => write!(f, "CASCADE"),
            Self::SetNull    => write!(f, "SET NULL"),
            Self::Restrict   => write!(f, "RESTRICT"),
            Self::SetDefault => write!(f, "SET DEFAULT"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GeneratedKind {
    Stored,
    Virtual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ForeignKeyRef {
    pub table:     &'static str,
    pub column:    &'static str,
    pub on_delete: FkAction,
    pub on_update: FkAction,
}

impl ForeignKeyRef {
    pub const fn new(table: &'static str, column: &'static str) -> Self {
        Self { table, column, on_delete: FkAction::NoAction, on_update: FkAction::NoAction }
    }

    pub const fn on_delete(mut self, action: FkAction) -> Self {
        self.on_delete = action;
        self
    }

    pub const fn on_update(mut self, action: FkAction) -> Self {
        self.on_update = action;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EntityConstraint {
    Unique(&'static [&'static str]),
    ForeignKey {
        columns:     &'static [&'static str],
        ref_table:   &'static str,
        ref_columns: &'static [&'static str],
        on_delete:   FkAction,
    },
    Check(&'static str),
    PrimaryKey(&'static [&'static str]),
}
