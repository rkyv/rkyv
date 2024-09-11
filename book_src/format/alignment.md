# Alignment

The _alignment_ of a type restricts where it can be located in memory to optimize hardware loads and
stores. Because rkyv creates references to values located in your serialized bytes, it has to ensure
that the references it creates are properly _aligned_ for the type.

> In order to perform arithmetic and logical operations on data, modern CPUs need to _load_ that
> data from memory into its registers. However, there's usually a hardware limitation on how the CPU
> can access that data: it can only access data starting at _word boundaries_. These words are the
> natural size for the CPU to work with; the word size is 4 bytes for 32-bit machines and 8 bytes
> for 64-bit machines. Imagine we had some data laid out like this:
>
> ```
> 0   4   8   C
> AAAABBBBCCCCDDDD
> ```
>
> On a 32-bit CPU, accesses could occur at any address that's a multiple of 4 bytes. For example,
> one could access `A` by loading 4 bytes from address 0, `B` by loading 4 bytes from address 4, and
> so on. This works great because our data is _aligned_ to word boundaries. _Unaligned_ data can
> throw a wrench in that:
>
> ```
> 0   4   8   C
> ..AAAABBBBCCCC
> ```
>
> Now if we want to load `A` into memory, we have to:
>
> 1. Load 4 bytes from address 0
> 2. Throw away the first two bytes
> 3. Load 4 bytes from address 4
> 4. Throw away the last two bytes
> 5. Combine our four bytes together
>
> That forces us to do twice as many loads _and_ perform some correction logic. That can have a real
> impact on our performance across the board, so we require all of our data to be properly aligned.

rkyv provides two main utilities for aligning byte buffers:

- `AlignedVec`, a higher-aligned drop-in replacement for `Vec<u8>`
- `Align`, a wrapper type which aligns its field to a 16-byte boundary

For most use cases, 16-aligned memory should be sufficient.

## In practice

rkyv's unchecked APIs have very basic alignment checks which always run in debug builds. These may
not catch every case, but using [validation](../validation.md) will always make sure that your data
is properly aligned.

### Common pitfalls

In some cases, your archived data may be prefixed by some extra data like the length of the buffer.
If this extra data misaligns the following data, then the buffer will have to have the prefixing
data removed before accessing it.

In other cases, your archived data may not be tight to the end of the buffer. Functions like
`access` rely on the end of the buffer being tight to the end of the data, and may miscalculate the
position of the archived data if it is not.
