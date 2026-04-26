//! Storage / file-system helpers — emit [`Operation`]s targeting
//! [`TargetKind::Blob`] or [`TargetKind::FileTree`].
//!
//! Storage operations are *just* DML against a non-tabular target, so this
//! module is a thin convenience layer that constructs the appropriate
//! `Insert` / `Query` / `Replace` / `Update` payload.

use dol_expr::{ExprArena, Interner};
use dol_ir::operation::{Insert, InsertSource, Query, Replace, ReplaceBody, Update};
use dol_ir::{Locator, Operation, Program, SchemaBinding, Target as IrTarget, TargetKind};
use smallvec::smallvec;

use crate::target::intern_symbol;

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
    let segments: smallvec::SmallVec<[dol_ir::Symbol; 2]> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|seg| intern_symbol(interner, seg))
        .collect();
    let (name, path_segs) = if let Some(last) = segments.last().copied() {
        let head: smallvec::SmallVec<[dol_ir::Symbol; 2]> =
            segments[..segments.len() - 1].iter().copied().collect();
        (last, head)
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

/// Emit `Operation::Update` against a file-tree path (move/rename — partial
/// mutation of the path attribute).
pub fn move_file(from: &str, to: &str) -> Program {
    use dol_expr::expr::{ExprNode, UpdateNode};
    let mut interner = Interner::new();
    let mut arena = ExprArena::new();
    let target = filetree_target(&mut interner, from);

    // Build an arena UpdateNode that records the destination as a
    // single-row literal value. The interpretation is target-specific;
    // here we encode `to` as a parameter slot for runtime binding.
    let target_str = interner.intern(from);
    let to_id = interner.intern(to);
    let lit_id = arena.alloc_lit(dol_core::Literal::String(std::borrow::Cow::Owned(
        to.to_string(),
    )));
    let val = arena.alloc(ExprNode::Lit(lit_id));
    let _ = to_id;
    let unode = UpdateNode {
        target: target_str,
        columns: smallvec![interner.intern("path")],
        values: smallvec![val],
        filter: dol_expr::ids::NULL_NODE,
        returning: smallvec![],
    };
    let uid = arena.alloc_update(unode);
    let body = arena.alloc(ExprNode::Update(uid));

    let op: Operation = Update { target, node: body }.into();
    Program::new(op, arena, interner)
}
