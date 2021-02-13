# Architecture

The core of rkyv is built around three core traits:
[`Write`](https://docs.rs/rkyv/latest/rkyv/trait.Write.html),
[`Archive`](https://docs.rs/rkyv/latest/rkyv/trait.Archive.html),
and [`Resolve`](https://docs.rs/rkyv/latest/rkyv/trait.Resolve.html).

## Write

Writers are types that accept bytes and write them in order to some output. The most basic example
of a writer might be a simple file. Writers can additionally provide the position of the next byte,
which is important for [relative pointers](relative-pointers.html).

## Archive

Archive types are able to convert themselves to their archived counterparts and write them to a
Writer. This happens in two steps:

1. Any dependencies of the type are written to the writer. For strings this would be the characters
of the string, for boxes it would be the boxed value, and for vectors it would be any contained
values. This is the archive step.
2. The value itself is written to the writer. For strings this would be the length and a pointer to
the characters, for boxes it would be a pointer to the boxed value, and for vectors it would be a
length and pointer to the archived values. This is the resolve step.

## Resolve

Archiveable types have a resolver type, which is just any information that needs to get carried from
the first step to the second. In most cases, a resolver is just the positions where the dependencies
of the type were written.

A good example of why resolvers are necessary is when archiving a tuple. Say we have two strings:

```rust
let value = ("hello".to_string(), "world".to_string());
```

The archived tuple needs to have both of the strings right next to each other:

```
0x0000      AA AA AA AA BB BB BB BB
0x0008      CC CC CC CC DD DD DD DD
```

A and B might be the length and pointer for the first string of the tuple, and C and D might be the
length and pointer for the second string.

When archiving, we might be tempted to archive and resolve the first string, then archive and
resolve the second one. But this might place the second string's bytes ("world") between the two!
Instead, we need to write out the bytes for both strings, and then finish archiving both of them.
The tuple doesn't know what information the strings need to finish archiving themselves, so they
have to provide it to the tuple through their Resolver.

This way, the tuple can:

1. Archive the first string (save the resolver)
2. Archive the second string (save the resolver)
3. Resolve the first string with its resolver
4. Resolve the second string with its resolver

And we're guaranteed that the two strings are placed right next to each other like we need.
