//! DOL IR — Statement enum and Backend trait (internal).

#![deny(unsafe_code)]

pub mod backend;
pub mod statement;

pub use backend::{Backend, BackendError};
pub use statement::Statement;
