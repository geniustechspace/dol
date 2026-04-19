//! DOL type system — the data structure and type layer.
//!
//! This module is the single source of truth for every type that flows through
//! DOL: in expressions, schema definitions, query bindings, result decoding,
//! backend lowering, and wire transport.
//!
//! # Layers
//!
//! | Layer | Types | Purpose |
//! |-------|-------|---------|
//! | **Values** | [`Value`], [`Literal`] | Carry actual data at runtime and in ASTs |
//! | **Descriptors** | [`DataType`], [`StructField`] | Describe the expected shape of a position |
//! | **Primitives** | [`Decimal`], [`Date`], [`Time`], [`Interval`], [`IpAddr`], [`MacAddr`], [`MacAddr8`], [`BitString`], [`geo::Point`], etc. | Structural sub-types with validated constructors |
//! | **Errors** | [`TypeError`] | All validation and conformance errors |
//!
//! # The `DataType` / `Value` contract
//!
//! [`DataType`] describes what a field or parameter expects.
//! [`Value`] is what actually arrives.
//! [`DataType::accepts`] is the bridge: it returns `Ok(())` if the value
//! satisfies the type, or a structured [`TypeError`] otherwise.
//!
//! # Type inventory
//!
//! | Category   | `DataType` variants | `Value` / `Literal` variants |
//! |------------|---------------------|------------------------------|
//! | Primitive  | `Null`, `Bool` | `Null`, `Bool` |
//! | Integer    | `Int8`–`Int128`, `UInt8`–`UInt128` | same |
//! | Float      | `Float32`, `Float64` | same |
//! | Decimal    | `Decimal { precision, scale }` | `Decimal` |
//! | Text       | `Char`, `Varchar`, `Text`, `Json`, `Xml` | `String`, `Json`, `Xml`, `Enum` |
//! | Binary     | `Binary`, `Varbinary`, `Uuid`, `Bit`, `Varbit` | `Bytes`, `Uuid`, `BitString` |
//! | Datetime   | `Date`, `Time`, `DateTime`, `TimestampTz`, `Interval` | same |
//! | Network    | `Inet`, `Cidr`, `MacAddr`, `MacAddr8` | `Inet`, `MacAddr`, `MacAddr8` |
//! | Geometric  | `Point`–`Polygon` | `Point`–`Polygon` |
//! | Composite  | `Array`, `Set`, `Map`, `Range`, `Tuple`, `Struct`, `Enum` | same + `Range` |
//! | Meta       | `TypeRef` | — |
//!
//! # Size guarantees (64-bit targets)
//!
//! ```text
//! size_of::<Value>()            == 24
//! size_of::<Literal<'static>>() == 32
//! ```
//!
//! These are enforced by tests in [`value`] and must not regress.

pub mod error;
pub mod datetime;
pub mod network;
pub mod geo;
pub mod numeric;
pub mod binary;
pub mod descriptor;
pub mod value;

// ─── Re-exports ───────────────────────────────────────────────────────────────

pub use error::TypeError;

pub use datetime::{Date, DateTime, Interval, Offset, Time, TimestampTz};
pub use network::{IpAddr, MacAddr, MacAddr8};
pub use geo::{Circle, Line, Path, Point, Polygon, Rect, Segment};
pub use numeric::Decimal;
pub use binary::BitString;

pub use descriptor::{DataType, StructField};

pub use value::{Literal, LiteralRange, Value, ValueRange};
