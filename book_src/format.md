# Format

The rkyv crate provides `Archive` implementations for common core and std types. In general they
follow the same format as derived implementations, but may differ in some cases. For more details
on the layouts of these types, see the
[`core_impl`](https://docs.rs/rkyv/latest/rkyv/core_impl/index.html) and
[`std_impl`](https://docs.rs/rkyv/latest/rkyv/std_impl/index.html) modules.

Types which derive `Archive` generate an archived version of the type where:

- Member types are replaced with their archived counterparts
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
struct ArchivedExample {
    a: u32,
    b: ArchivedString,
    c: ArchivedBox<(u32, ArchivedString)>,
}
```

With the strict feature, these structs are additionally annotated with `#[repr(C)]` for guaranteed
portability and stability.