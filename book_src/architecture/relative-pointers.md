# Relative pointers

Relative pointers are the bread and butter of total zero-copy deserialization, completely replacing
the use of normal pointers. But why can't we use normal pointers?

Consider some zero-copy data on disk. Before we can use it, we need to load it into memory. But we
can't control _where_ in memory it gets loaded. Every time we load it, it could be located at a
different address, and therefore the objects inside of it will be located at a different address.

> One of the major reasons for this is actually *security*. Every time you run your program, it may
> run in a completely different random location in memory. This is called
> [address space layout randomization](https://en.wikipedia.org/wiki/Address_space_layout_randomization)
> and it helps prevent exploitation of memory corruption vulnerabilities.
>
> At most, we can only control the *alignment* of our zero-copy data, so we need to work within
> those constraints.

This means that we can't store any pointers to that data, inside of it or outside of it. As soon as
we reload the data, it might not be at the same address. That would leave our pointers dangling, and
would almost definitely result in memory access violations. Some other libraries like
[abomonation](https://github.com/TimelyDataflow/abomonation) store some extra data and perform a
fast fixup step that takes the place of deserialization, but we can do better.

> In order to perform that fixup step, abomonation requires that the buffer has a *mutable backing*.
> This is okay for many use cases, but there are also cases where we won't be able to mutate our
> buffer. One example is if we used
> [memory-mapped files](https://en.wikipedia.org/wiki/Memory-mapped_file).

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

rkyv's implementation of relative pointers is the `RelPtr` type.
