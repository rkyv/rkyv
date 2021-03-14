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

rkyv does not natively support reconciling endianness. That means that little-endian systems will
write little-endian archives and big-endian systems will write big-endian archives.

You can still write your own custom type wrappers and use those to achieve cross-endian archives.
For example, a wrapper type could always serialize integers as little-endian and then provide a
getter that converts the on-disk representation to native endianness. Deserialization would also
simply convert to native endianness.

## Is rkyv cross-platform?

Yes, but with caveats:

- You can use it on either little- or big-endian systems, but it will always use native endianness.
  See "How does rkyv handle endianness?" for more information.
- rkyv has not been widely tested and there may be bugs that need to get fixed.

That said, it is explicitly a goal of rkyv to be cross-platform within reason (and that reason is
very wide).

## Can I use this in embedded and `#[no_std]` environments?

Yes, but be on the lookout for bugs because these environments have not been thoroughly tested.

# Safety

## Isn't this very unsafe if you access untrusted data?

Yes, *but* you can still access untrusted data if you validate the archive first with
[bytecheck](https://github.com/djkoloski/bytecheck). It's an extra step and requires a decent amount
of processing, but it's typically still less than the cost of deserializing using a traditional
format.

## Doesn't that mean I basically can't use it anywhere?

**No**. There are many other ways you can verify your data, for example with checksums and signed
buffers. If your security model prevents you from using those techniques, then it definitely
prevents you from using most or all zero-copy deserialization solutions and you're stuck with slower
traditional serialization.

## Isn't it kind of deceptive to say rkyv is fast and then require validation?

The regular (fast) path to access archived data is marked as `unsafe`. This doesn't mean that it's
unusable, it means that it's only safe to call if you can verify its preconditions:

> This is only safe to call if the value is archived at the given position in the byte array.

As long as you can (reasonably) guarantee that, then accessing the archive is safe. Not every
archive needs to be validated, and you can use a variety of different techniques to guarantee data
integrity and security.

Validation is provided for archives because it's still typically faster to validate and access an
archive than it is to deserialize traditional formats. Additionally, it has more uses than just
checking potentially malicious archives.

## Isn't it unfair if competitors like Cap'n Proto validate their archives but you don't?

Cap'n Proto does validate their archives before use, but it comes with the
[same security guarantees](https://capnproto.org/faq.html#security) as rkyv. The primary difference
is that Cap'n Proto always validates their data and rkyv gives the user the decision. Rust's
safe/unsafe system makes this choice explicit and different users will choose differently.

FlatBuffers [also has verification](https://github.com/dvidelabs/flatcc/blob/master/doc/security.md)
(for rust?) but they have the same set of warnings and caveats as rkyv and Cap'n Proto.
