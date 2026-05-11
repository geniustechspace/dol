//! Function metadata: [`Arity`], [`FuncKind`], [`FuncDef`], the [`DolFunc`]
//! trait, and the [`define_func!`] macro.
//!
//! This module is the single source of truth for what a DOL function *is*.
//! It deliberately contains no string-constant catalogue — that lives in
//! [`super::registry`], where each entry is a zero-sized struct that implements
//! [`DolFunc`].

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use dol_core::strings::Name;

use super::super::Expr;

// ─────────────────────────────────────────────────────────────────────────────
// Arity
// ─────────────────────────────────────────────────────────────────────────────

/// Describes the expected argument count for a DOL function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Arity {
    /// Exactly `n` arguments required.
    Exact(u8),
    /// At least `n` arguments required (variadic tail).
    AtLeast(u8),
    /// Between `low` and `high` arguments inclusive.
    Range(u8, u8),
    /// Any number of arguments accepted (no validation).
    Any,
}

/// Error returned by [`FuncDef::validate_arity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArityError {
    /// The name of the function that failed validation.
    pub func_name: String,
    /// The constraint declared for this function.
    pub expected: Arity,
    /// The number of arguments actually supplied.
    pub actual: usize,
}

impl fmt::Display for ArityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "function '{}': expected {} argument(s), got {}",
            self.func_name,
            match self.expected {
                Arity::Exact(n) => format!("exactly {n}"),
                Arity::AtLeast(n) => format!("at least {n}"),
                Arity::Range(low, high) => format!("{low}..={high}"),
                Arity::Any => "any number of".to_string(),
            },
            self.actual,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FuncKind
// ─────────────────────────────────────────────────────────────────────────────

/// Broad category of a DOL function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum FuncKind {
    /// A scalar function (e.g. `LOWER`, `ABS`, `ROUND`).
    Scalar,
    /// An aggregate function (e.g. `COUNT`, `SUM`, `AVG`).
    Aggregate,
    /// A window / ranking function (e.g. `ROW_NUMBER`, `RANK`).
    Window,
}

// ─────────────────────────────────────────────────────────────────────────────
// FuncDef
// ─────────────────────────────────────────────────────────────────────────────

/// Runtime carrier for a function definition: name + arity + kind.
///
/// For well-known functions, prefer constructing via [`DolFunc::def()`] — it
/// is allocation-free and always in sync with the registry.  Use
/// [`FuncDef::custom`] / [`FuncDef::custom_with`] only for user-defined or
/// backend-specific functions that have no registry entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FuncDef {
    name: Name,
    arity: Arity,
    kind: FuncKind,
}

impl FuncDef {
    /// Construct from compile-time-known constants.  Zero allocation.
    ///
    /// Prefer [`DolFunc::def()`] over calling this directly.
    pub const fn new_static(name: &'static str, arity: Arity, kind: FuncKind) -> Self {
        Self {
            name: Name::Static(name),
            arity,
            kind,
        }
    }

    /// Construct a custom function definition.
    ///
    /// Defaults to [`Arity::Any`] and [`FuncKind::Scalar`].
    pub fn custom(name: impl Into<Box<str>>) -> Self {
        Self {
            name: Name::Owned(name.into()),
            arity: Arity::Any,
            kind: FuncKind::Scalar,
        }
    }

    /// Construct a custom function with explicit arity and kind.
    pub fn custom_with(name: impl Into<Box<str>>, arity: Arity, kind: FuncKind) -> Self {
        Self {
            name: Name::Owned(name.into()),
            arity,
            kind,
        }
    }

    /// The function name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// The arity constraint.
    pub fn arity(&self) -> Arity {
        self.arity
    }

    /// The function category.
    pub fn kind(&self) -> FuncKind {
        self.kind
    }

    /// Returns `Ok(())` when `arg_count` satisfies the arity constraint, or an
    /// [`ArityError`] otherwise.
    pub fn validate_arity(&self, arg_count: usize) -> Result<(), ArityError> {
        let ok = match self.arity {
            Arity::Exact(n) => arg_count == n as usize,
            Arity::AtLeast(n) => arg_count >= n as usize,
            Arity::Range(low, high) => arg_count >= low as usize && arg_count <= high as usize,
            Arity::Any => true,
        };
        if ok {
            Ok(())
        } else {
            Err(ArityError {
                func_name: self.name().to_string(),
                expected: self.arity,
                actual: arg_count,
            })
        }
    }
}

impl fmt::Display for FuncDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialEq<&str> for FuncDef {
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

impl PartialEq<FuncDef> for &str {
    fn eq(&self, other: &FuncDef) -> bool {
        *self == other.name()
    }
}

impl From<&FuncDef> for Cow<'_, str> {
    fn from(def: &FuncDef) -> Self {
        Cow::Owned(def.name().to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DolFunc trait
// ─────────────────────────────────────────────────────────────────────────────

/// Implemented by zero-sized structs representing well-known DOL functions.
///
/// Use [`define_func!`] to generate the implementation.  Backends extend
/// individual registry structs with additional backend-specific traits:
///
/// ```ignore
/// // in the postgres backend crate
/// pub trait PostgresFunc: DolFunc {
///     fn pg_name() -> &'static str { Self::NAME }
/// }
/// impl PostgresFunc for dol_expr::tree::func::registry::PadLeft {
///     fn pg_name() -> &'static str { "LPAD" }
/// }
/// ```
pub trait DolFunc: Sized + 'static {
    /// Canonical DOL name (e.g. `"LOWER"`).
    const NAME: &'static str;
    /// Arity constraint.
    const ARITY: Arity;
    /// Function category.
    const KIND: FuncKind;

    /// Build the [`FuncDef`] runtime carrier from trait constants.
    ///
    /// Allocation-free; suitable for `const` contexts via
    /// [`FuncDef::new_static`].
    fn def() -> FuncDef {
        FuncDef::new_static(Self::NAME, Self::ARITY, Self::KIND)
    }

    /// Build a function-call [`Expr`] node with the given arguments.
    ///
    /// Arity is **not** validated here; call [`FuncDef::validate_arity`]
    /// explicitly on the result's `name` field when you need a checked path
    /// (e.g. in `try_build`).
    fn call<'a>(args: Vec<Expr<'a>>) -> Expr<'a> {
        Expr::Func {
            name: Self::def(),
            args,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// define_func! macro
// ─────────────────────────────────────────────────────────────────────────────

/// Declare a zero-sized struct that implements [`DolFunc`].
///
/// # Forms
///
/// **Base** — struct + `DolFunc` impl only.  Use `Foo::call(args)` to build
/// an [`Expr`] node.
/// ```ignore
/// define_func!(Lower, "LOWER", Arity::Exact(1), FuncKind::Scalar);
/// ```
///
/// **Zero-arg builder** — also emits `pub fn now<'a>() -> Expr<'a>`.
/// ```ignore
/// define_func!(Now, "NOW", Arity::Exact(0), FuncKind::Scalar,
///     builder = now());
/// ```
///
/// **Fixed-arity builder** — emits a typed free function with one `impl
/// Into<Expr<'a>>` parameter per argument name.
/// ```ignore
/// define_func!(Upper, "UPPER", Arity::Exact(1), FuncKind::Scalar,
///     builder = upper(s));
///
/// define_func!(Nullif, "NULLIF", Arity::Exact(2), FuncKind::Scalar,
///     builder = nullif(a, b));
/// ```
///
/// **Variadic builder** — emits a free function taking `Vec<Expr<'a>>`.
/// ```ignore
/// define_func!(Concat, "CONCAT", Arity::AtLeast(1), FuncKind::Scalar,
///     builder = concat(*));
/// ```
///
/// # Notes
///
/// - Builder functions are emitted in the same module as the `define_func!`
///   invocation; re-export them from `func::mod` as needed.
/// - For functions with optional trailing arguments (e.g. `LAG`, `ROUND`),
///   write a hand-rolled builder in `func::mod` rather than forcing an
///   optional-arg pattern through the macro.
/// - Backends receive no builder — they use `registry::Foo::call(args)` or
///   impl their own extension trait on the zero-sized struct.
#[macro_export]
macro_rules! define_func {
    // ── base: struct + DolFunc impl only ─────────────────────────────────
    ($struct_name:ident, $name:expr, $arity:expr, $kind:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $struct_name;

        impl $crate::tree::func::meta::DolFunc for $struct_name {
            const NAME: &'static str = $name;
            const ARITY: $crate::tree::func::meta::Arity = $arity;
            const KIND: $crate::tree::func::meta::FuncKind = $kind;
        }
    };

    // ── zero-arg builder ──────────────────────────────────────────────────
    ($struct_name:ident, $name:expr, $arity:expr, $kind:expr,
     builder = $fn_name:ident()) => {
        $crate::define_func!($struct_name, $name, $arity, $kind);

        pub fn $fn_name<'a>() -> $crate::tree::Expr<'a> {
            <$struct_name as $crate::tree::func::meta::DolFunc>::call(::alloc::vec![])
        }
    };

    // ── fixed-arity builder (1 … N positional args) ───────────────────────
    ($struct_name:ident, $name:expr, $arity:expr, $kind:expr,
     builder = $fn_name:ident($($arg:ident),+)) => {
        $crate::define_func!($struct_name, $name, $arity, $kind);

        pub fn $fn_name<'a>(
            $($arg: impl Into<$crate::tree::Expr<'a>>),+
        ) -> $crate::tree::Expr<'a> {
            <$struct_name as $crate::tree::func::meta::DolFunc>::call(
                ::alloc::vec![$($arg.into()),+],
            )
        }
    };

    // ── variadic builder (Vec<Expr>) ──────────────────────────────────────
    ($struct_name:ident, $name:expr, $arity:expr, $kind:expr,
     builder = $fn_name:ident(*)) => {
        $crate::define_func!($struct_name, $name, $arity, $kind);

        pub fn $fn_name<'a>(
            args: ::alloc::vec::Vec<$crate::tree::Expr<'a>>,
        ) -> $crate::tree::Expr<'a> {
            <$struct_name as $crate::tree::func::meta::DolFunc>::call(args)
        }
    };
}
