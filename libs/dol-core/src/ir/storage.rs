//! Backward compatibility — re-exports from [`crate::op::storage`].

pub use crate::op::storage::*;

pub type PutObjectIR<'a> = crate::op::PutObject<'a>;
pub type GetObjectIR = crate::op::GetObject;
pub type ListObjectsIR = crate::op::ListObjects;
pub type ReadFileIR = crate::op::ReadFile;
pub type WriteFileIR<'a> = crate::op::WriteFile<'a>;
pub type MoveFileIR = crate::op::MoveFile;
