# Relative pointers

Relative pointers are the bread and butter of total zero-copy deserialization, completely replacing
the use of normal pointers. But why can't we use normal pointers?

Consider some zero-copy data on disc. Before we can use it, we need to load it into memory. But we
can't control _where_ in memory it gets loaded. Every time we load it, it could be located at a
different address, and therefore the objects inside of it will be located at a different address.
This means that we can't store any pointers to that data, inside of it or outside of it. Some
libraries like [abomonation](https://github.com/TimelyDataflow/abomonation) store some extra data
and perform a fast fixup step that takes the place of deserialization, but we can do better.

While normal pointers hold an absolute address in memory, relative pointers hold an offset to an address. This changes how
the pointer behaves under moves:

| Pointer   | Self is moved                     | Self and target are moved                         |
|-----------|-----------------------------------|---------------------------------------------------|
| Absolute  | ✅ Target is still at address      | ❌ Target no longer at address                     |
| Relative  | ❌ Relative distance has changed   | ✅ Self and target same relative distance apart    |

This is exactly the property we need to build data structures with total zero-copy deserialization.
By using relative pointers, we can load data at any position in memory and still have valid pointers
inside of it. Relative pointers don't require write access to memory either, so we can memory map
entire files and instantly have access to their data in a structured manner.

rkyv's implementation of relative pointers is the
[`RelPtr`](https://docs.rs/rkyv/latest/rkyv/struct.RelPtr.html) type.