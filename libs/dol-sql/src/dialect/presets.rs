/// Preset dialect configurations for all supported databases.
use super::{
    ArrayLiteralStyle, Dialect, JsonAccessStyle, StoreKind, concat::ConcatStyle,
    ddl::DdlCapabilities, features::DialectFeatures, locking::LockingCapabilities,
    pagination::PaginationStyle, param::ParamStyle, quoting::QuoteStyle, returning::ReturningStyle,
    types::TypeMap, upsert::UpsertStyle,
};

impl Dialect {
    /// PostgreSQL 12+ preset.
    pub fn postgres() -> Self {
        Self {
            name: "postgresql".into(),
            param_style: ParamStyle::postgres(),
            quote_style: QuoteStyle::DoubleQuote,
            type_map: TypeMap::postgres(),
            pagination: PaginationStyle::LimitOffset,
            upsert_style: UpsertStyle::OnConflict,
            returning_style: ReturningStyle::Returning,
            locking: LockingCapabilities::postgres(),
            ddl: DdlCapabilities::postgres(),
            features: DialectFeatures {
                distinct_on: true,
                ilike: true,
                array_any: true,
                nulls_ordering: true,
                anonymous_blocks: true,
                schemas: true,
                cte: true,
                window_functions: true,
                lateral_join: true,
                on_conflict: true,
            },
            bool_true: "TRUE".into(),
            bool_false: "FALSE".into(),
            concat_style: ConcatStyle::PipeOperator,
            store_kind: StoreKind::Sql,
            json_access: JsonAccessStyle::ArrowOperator,
            array_literal_style: ArrayLiteralStyle::ArrayKeyword,
        }
    }

    /// MySQL 8+ preset.
    pub fn mysql() -> Self {
        Self {
            name: "mysql".into(),
            param_style: ParamStyle::mysql(),
            quote_style: QuoteStyle::Backtick,
            type_map: TypeMap::mysql(),
            pagination: PaginationStyle::LimitOffset,
            upsert_style: UpsertStyle::OnDuplicateKey,
            returning_style: ReturningStyle::Unsupported,
            locking: LockingCapabilities::mysql(),
            ddl: DdlCapabilities::mysql(),
            features: DialectFeatures {
                distinct_on: false,
                ilike: false,
                array_any: false,
                nulls_ordering: false,
                anonymous_blocks: false,
                schemas: false, // MySQL uses databases, not schemas
                cte: true,
                window_functions: true,
                lateral_join: true,
                on_conflict: false,
            },
            bool_true: "1".into(),
            bool_false: "0".into(),
            concat_style: ConcatStyle::ConcatFunction,
            store_kind: StoreKind::Sql,
            json_access: JsonAccessStyle::JsonExtractFunction,
            array_literal_style: ArrayLiteralStyle::JsonArrayFunction,
        }
    }

    /// MariaDB 10.5+ preset.
    pub fn mariadb() -> Self {
        let mut d = Self::mysql();
        d.name = "mariadb".into();
        d.returning_style = ReturningStyle::Returning;
        d
    }

    /// SQLite 3.35+ preset.
    pub fn sqlite() -> Self {
        Self {
            name: "sqlite".into(),
            param_style: ParamStyle::mysql(), // SQLite also uses `?`
            quote_style: QuoteStyle::DoubleQuote,
            type_map: TypeMap::sqlite(),
            pagination: PaginationStyle::LimitOffset,
            upsert_style: UpsertStyle::OnConflict,
            returning_style: ReturningStyle::Returning,
            locking: LockingCapabilities::sqlite(),
            ddl: DdlCapabilities::sqlite(),
            features: DialectFeatures {
                distinct_on: false,
                ilike: false,
                array_any: false,
                nulls_ordering: true,
                anonymous_blocks: false,
                schemas: false,
                cte: true,
                window_functions: true,
                lateral_join: false,
                on_conflict: true,
            },
            bool_true: "1".into(),
            bool_false: "0".into(),
            concat_style: ConcatStyle::PipeOperator,
            store_kind: StoreKind::Sql,
            json_access: JsonAccessStyle::JsonExtractFunction,
            array_literal_style: ArrayLiteralStyle::JsonArrayFunction,
        }
    }

    /// Microsoft SQL Server 2016+ preset.
    pub fn mssql() -> Self {
        Self {
            name: "mssql".into(),
            param_style: ParamStyle::mssql(),
            quote_style: QuoteStyle::Bracket,
            type_map: TypeMap::mssql(),
            pagination: PaginationStyle::OffsetFetch,
            upsert_style: UpsertStyle::Merge,
            returning_style: ReturningStyle::OutputInserted,
            locking: LockingCapabilities::mssql(),
            ddl: DdlCapabilities::mssql(),
            features: DialectFeatures {
                distinct_on: false,
                ilike: false,
                array_any: false,
                nulls_ordering: false,
                anonymous_blocks: false,
                schemas: true,
                cte: true,
                window_functions: true,
                lateral_join: true, // CROSS APPLY / OUTER APPLY
                on_conflict: false,
            },
            bool_true: "1".into(),
            bool_false: "0".into(),
            concat_style: ConcatStyle::PlusOperator,
            store_kind: StoreKind::Sql,
            json_access: JsonAccessStyle::JsonValueFunction,
            array_literal_style: ArrayLiteralStyle::Unsupported,
        }
    }

    /// Oracle 12c+ preset.
    pub fn oracle() -> Self {
        Self {
            name: "oracle".into(),
            param_style: ParamStyle::oracle(),
            quote_style: QuoteStyle::DoubleQuote,
            type_map: TypeMap::oracle(),
            pagination: PaginationStyle::OffsetFetch,
            upsert_style: UpsertStyle::Merge,
            returning_style: ReturningStyle::ReturningInto,
            locking: LockingCapabilities::oracle(),
            ddl: DdlCapabilities::oracle(),
            features: DialectFeatures {
                distinct_on: false,
                ilike: false,
                array_any: false,
                nulls_ordering: true,
                anonymous_blocks: true,
                schemas: true,
                cte: true,
                window_functions: true,
                lateral_join: true,
                on_conflict: false,
            },
            bool_true: "1".into(),
            bool_false: "0".into(),
            concat_style: ConcatStyle::PipeOperator,
            store_kind: StoreKind::Sql,
            json_access: JsonAccessStyle::Unsupported,
            array_literal_style: ArrayLiteralStyle::Unsupported,
        }
    }

    /// CockroachDB preset (PostgreSQL-compatible with minor differences).
    pub fn cockroachdb() -> Self {
        let mut d = Self::postgres();
        d.name = "cockroachdb".into();
        // CockroachDB doesn't support CONCURRENTLY
        d.ddl.index_concurrently = false;
        // CockroachDB doesn't support anonymous DO blocks
        d.features.anonymous_blocks = false;
        d
    }
}
