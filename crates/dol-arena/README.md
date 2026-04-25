# `dol-arena`

Generic arena + interner + typed ids. Pure infrastructure used by every DOL
layer that allocates AST/IR nodes — this crate has **no DOL semantics**:
it knows nothing about expressions, statements, schemas, or wire formats.
Higher layers parameterise its generics with their own marker types.

## Design

- `Arena<T>` is a contiguous `Vec<T>` keyed by `Id<T, Tag>`. Ids are `u32`
  indices; they cannot be confused across arenas because the phantom `Tag`
  type tags each id.
- `Interner` gives every unique byte sequence a stable `StrId`. Backed by
  `hashbrown` with the default hasher, `Arc<str>` storage, and a single
  shared allocation per unique string.
- `StrId` is a newtype `u32` with a 24-bit index + 8-bit user-defined `kind`
  byte for cheap categorisation (e.g. "identifier" vs. "literal").

## Sizes (asserted in tests)

```text
size_of::<Id<u8, ()>>() == 4
size_of::<StrId>()      == 4
```

## Features

| feature | default | effect                                                |
| ------- | :-----: | ----------------------------------------------------- |
| `serde` |         | `Serialize` / `Deserialize` for `Id`, `StrId`, arenas |
| `std`   |         | implements `std::error::Error` for arena errors       |

Default build is `no_std + alloc`.

## Example

```rust,ignore
use dol_arena::{Arena, Id};

struct Node;
let mut a: Arena<Node, ()> = Arena::new();
let id: Id<Node, ()> = a.push(Node);
```

See the rustdoc for the full API.
