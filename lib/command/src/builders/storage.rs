//! Storage / file-system helpers — emit [`Operation`]s targeting
//! [`TargetKind::Blob`] or [`TargetKind::FileTree`].
//!
//! Storage operations are *just* DML against a non-tabular target, so this
//! module is a thin convenience layer that constructs the appropriate
//! `Insert` / `Query` / `Replace` / `Update` payload.

use crate::operation::Operation;
use crate::operation::{Insert, InsertSource, Query, Replace, ReplaceBody};
use crate::program::Program;
use crate::target::{Locator, SchemaBinding, Target as IrTarget, TargetKind};
use dol_expr::{ExprArena, Interner};
use smallvec::smallvec;

use super::target::intern_symbol;

fn blob_target(interner: &mut Interner, bucket: &str, key: &str) -> IrTarget {
    let bucket_sym = intern_symbol(interner, bucket);
    let key_sym = intern_symbol(interner, key);
    IrTarget {
        kind: TargetKind::Blob,
        locator: Locator {
            namespace: Some(bucket_sym),
            name: key_sym,
            path: smallvec![],
        },
        alias: None,
        schema: SchemaBinding::Opaque,
    }
}

fn filetree_target(interner: &mut Interner, path: &str) -> IrTarget {
    let segments: smallvec::SmallVec<[crate::target::Symbol; 2]> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|seg| intern_symbol(interner, seg))
        .collect();
    let (name, path_segs) = if let Some((last, head)) = segments.split_last() {
        let head: smallvec::SmallVec<[crate::target::Symbol; 2]> = head.iter().copied().collect();
        (*last, head)
    } else {
        (intern_symbol(interner, ""), smallvec![])
    };
    IrTarget {
        kind: TargetKind::FileTree,
        locator: Locator {
            namespace: None,
            name,
            path: path_segs,
        },
        alias: None,
        schema: SchemaBinding::Opaque,
    }
}

/// Emit `Operation::Insert` against a `TargetKind::Blob` target with the
/// payload supplied at execution time (`InsertSource::Bindings`).
pub fn put_blob(bucket: &str, key: &str) -> Program {
    let mut interner = Interner::new();
    let target = blob_target(&mut interner, bucket, key);
    let op: Operation = Insert {
        target,
        source: InsertSource::Bindings,
        returning: None,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Insert` against a `TargetKind::Blob` target reading from
/// a server-side path.
pub fn put_blob_from_path(bucket: &str, key: &str, source_path: &str) -> Program {
    let mut interner = Interner::new();
    let target = blob_target(&mut interner, bucket, key);
    let path_sym = intern_symbol(&mut interner, source_path);
    let op: Operation = Insert {
        target,
        source: InsertSource::FromPath(path_sym),
        returning: None,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Query` against a `TargetKind::Blob` (object read).
pub fn get_blob(bucket: &str, key: &str) -> Program {
    let mut interner = Interner::new();
    let target = blob_target(&mut interner, bucket, key);
    let op: Operation = Query { target, node: None }.into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Query` against a bucket (object listing).
pub fn list_blobs(bucket: &str) -> Program {
    let mut interner = Interner::new();
    let bucket_sym = intern_symbol(&mut interner, bucket);
    let target = IrTarget {
        kind: TargetKind::Blob,
        locator: Locator::new(bucket_sym),
        alias: None,
        schema: SchemaBinding::Opaque,
    };
    let op: Operation = Query { target, node: None }.into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Query` against a `TargetKind::FileTree` path (file read).
pub fn read_file(path: &str) -> Program {
    let mut interner = Interner::new();
    let target = filetree_target(&mut interner, path);
    let op: Operation = Query { target, node: None }.into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Replace` against a `TargetKind::FileTree` path (file
/// write — full overwrite).
pub fn write_file(path: &str) -> Program {
    let mut interner = Interner::new();
    let target = filetree_target(&mut interner, path);
    let op: Operation = Replace {
        target,
        body: ReplaceBody::Bindings,
        filter: None,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Replace` against a file-tree path reading from a
/// server-side source path.
pub fn write_file_from_path(dest: &str, source_path: &str) -> Program {
    let mut interner = Interner::new();
    let target = filetree_target(&mut interner, dest);
    let path_sym = intern_symbol(&mut interner, source_path);
    let op: Operation = Replace {
        target,
        body: ReplaceBody::FromPath(path_sym),
        filter: None,
    }
    .into();
    Program::new(op, ExprArena::new(), interner)
}

/// Emit `Operation::Schema` with `verb = Rename` against a `TargetKind::FileTree`
/// path — a faithful file-tree rename rather than the previous placeholder
/// `Update` against the `"path"` column.
///
/// # Examples
///
/// ```
/// use dol_command::builders::move_file;
/// use dol_command::operation::{Operation, OpKind, SchemaOp, StructuralVerb};
/// use dol_command::target::TargetKind;
///
/// let prog = move_file("fs/old/name.txt", "new-name.txt");
/// assert_eq!(prog.operations.len(), 1);
/// assert_eq!(prog.operations[0].kind(), OpKind::Schema);
/// match &prog.operations[0] {
///     Operation::Schema(op) => {
///         assert_eq!(op.verb, StructuralVerb::Rename);
///         assert_eq!(op.target.kind, TargetKind::FileTree);
///         assert!(op.new_name.is_some());
///     }
///     _ => unreachable!(),
/// }
/// ```
pub fn move_file(from: &str, to: &str) -> Program {
    use crate::operation::SchemaOp;

    let mut interner = Interner::new();
    let target = filetree_target(&mut interner, from);
    let new_name = intern_symbol(&mut interner, to);
    let op: Operation = SchemaOp::rename(target, new_name).into();
    Program::new(op, ExprArena::new(), interner)
}
