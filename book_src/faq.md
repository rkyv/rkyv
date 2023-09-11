# FAQ

Because it's so different from traditional serialization systems, a lot of people have questions
about rkyv. This is meant to serve as a comprehensive, centralized source for answers.

## How is rkyv zero-copy? It definitely copies the archive into memory.

Traditional serialization works in two steps:

1. Read the data from disk into a buffer (maybe in pieces)
2. Process the data in the buffer into the deserialized data structure

The copy happens when the data in the buffer ends up duplicated in the data structure. Zero-copy
deserialization doesn't deserialize the buffer into a separate structure and thus avoids this copy.

You can actually even avoid reading the data from disk into a buffer in most environments by using
memory mapping.

## How does rkyv handle endianness?

rkyv supports three endiannesses: native, little, and big. Native endianness will be either little
or big, but removes the abstraction layer to more easily work with the underlying types.

You can enable specific endiannesses with the `little_endian` and `big_endian` features.

## Is rkyv cross-platform?

Yes, but rkyv has been tested mostly on x86 machines and wasm. There may be bugs that need to get
fixed for other architectures.

## Can I use this in embedded and `#[no_std]` environments?

Yes, disable the `std` feature for `no_std`. You can additionally disable the `alloc` feature to
disable all memory allocation capabilities.

# Safety

## Isn't this very unsafe if you access untrusted data?

Yes, *but* you can still access untrusted data if you validate the archive first with
[bytecheck](https://github.com/rkyv/bytecheck). It's an extra step, but it's usually still less
than the cost of deserializing using a traditional format. rkyv has proven to round-trip faster than
bincode for all tested use cases.

## Doesn't that mean I always have to validate?

**No**. There are many other ways you can verify your data, for example with checksums and signed
buffers.

## Isn't it kind of deceptive to say rkyv is fast and then require validation?

The fastest path to access archived data is marked as `unsafe`. This doesn't mean that it's
unusable, it means that it's only safe to call if you can verify its preconditions:

> The value must be archived at the given position in the byte array.

As long as you can (reasonably) guarantee that, then accessing the archive is safe. Not every
archive needs to be validated, and you can use a variety of different techniques to guarantee data
integrity and security.

Even if you do need to always validate your data before accessing it, validation is always faster
than deserializing with other high-performance formats. A round-trip is still faster, even though
it's not by the same margins.
