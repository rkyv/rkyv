# Format

Types which derive `Archive` generate an archived version of the type where:

- Member types are replaced with their archived counterparts
- Structs are `#[repr(C)]`.
- Enums have `#[repr(N)]` where N is `u8`, `u16`, `u32`, `u64`, or `u128`, choosing the smallest
possible type that can represent all of the variants.

For example, a struct like:

```rust
struct Example {
    a: u32,
    b: String,
    c: Box<(u32, String)>,
}
```

Would have the archived counterpart:

```rust
#[repr(C)]
struct ArchivedExample {
    a: u32_le,
    b: ArchivedString,
    c: ArchivedBox<(u32_le, ArchivedString)>,
}
```

With the `little_endian` feature enabled.

rkyv provides `Archive` implementations for common core and std types by
default. In general they follow the same format as derived implementations, but
may differ in some cases. For example, `ArchivedString` performs a small string
optimization which helps reduce memory use.

## Object order

rkyv lays out subobjects in depth-first order from the leaves to the root. This means that the root
object is stored at the end of the buffer, not the beginning. For example, this tree:

```
  a
 / \
b   c
   / \
  d   e
```

would be laid out like this in the buffer:

```
b d e c a
```

from this serialization order:

```
a -> b
a -> c -> d
a -> c -> e
a -> c
a
```

This deterministic layout means that you don't need to store the position of the root object in most
cases. As long as your buffer ends right at the end of your root object, you can use
[`archived_root`](https://docs.rs/rkyv/0.7.1/rkyv/util/fn.archived_root.html) with your buffer.
