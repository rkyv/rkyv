# Serialize

Types implement `Serialize` separately from `Archive`. `Serialize` creates a resolver for some
object, then `Archive` turns the value and that resolver into an archived type. Having a separate
`Serialize` trait is necessary because although a type may have only one archived representation,
it may support many different types of _serializers_ which fulfill its requirements.

> The `Serialize` trait is parameterized over the *serializer*. The serializer is just a mutable
> object that helps the type serialize itself. The most basic types like `u32` or `char` don't
> *bound* their serializer type because they can serialize themselves with any kind of serializer.
> More complex types like `Box` and `String` require a serializer that implements `Writer`, and even
> more complex types like `Rc` and `Vec` require a serializer that additionally implements `Sharing`
> or `Allocator`.

Unlike `Serialize`, `Archive` doesn't parameterize over the serializer used to make it. It shouldn't
matter what serializer a resolver was made with, only that it's made correctly.

## Serializer

rkyv provides default serializers which can serialize all standard library types, as well as
components which can be combined into custom-built serializers. By combining rkyv's provided
components, serializers can be customized for high-performance, no-std, and custom allocation.

When using the high-level API, a `HighSerializer` provides a good balance of flexibility and
performance by default. When using the low-level API, a `LowSerializer` does the same without any
allocations. You can make custom serializers using the `Serializer` combinator, or by writing your
own from scratch.

rkyv comes with a few primary serializer traits built-in:

### Positional

This core serializer trait provides positional information during serialization. Because types need
to know the relative distance between objects, the `Positional` trait provides the current position
of the "write head" of the serializer. Resolvers will often store the _position_ of some serialized
data so that a relative pointer can be calculated to it during `resolve`.

### Writer

`Writer` accepts byte slices and writes them to some output. It is similar to the standard library's
`Write` trait, but rkyv's `Writer` trait works in no-std contexts. In rkyv, writers are always
_write-forward_ - they never backtrack and rewrite data later. This makes it possible for writers to
eagerly sink bytes to disk or the network without having to first buffer the entire message.

Several kinds of `Writer`s are supported by default:
- `Vec<u8>`
- `AlignedVec`, which is a highly-aligned vector of bytes. This is the writer rkyv uses by default
  in most cases.
- `Buffer`, which supports no-std use cases (for example, writing into fixed-size stack memory).
- Types which implement `std::io::Write` can be adapted into a `Writer` by wrapping them in the
  `IoWriter` type.

### Allocator

Many types require temporarily-allocated space during serialization. This space is used temporarily,
and then returned to the serializer before serialization finishes. For example, `Vec` might request
a dynamically-sized allocation to store the resolvers for its elements until it finishes serializing
all of them. Allocating memory from the serializer allows the same bytes to be efficiently reused
many times, which reduces the number of slow memory allocations performed during serialization.

### Sharing

rkyv serializes shared pointers like `Rc` and `Arc` and can control whether they are de-duplicated.
The `Sharing` trait provides some mutable state on the serializer which keeps track of which shared
pointers have been serialized so far, and can instruct repeated shared pointers to point to a
previously-serialized instance. This also allows rkyv to preserve shared pointers during zero-copy
access and deserialization.
