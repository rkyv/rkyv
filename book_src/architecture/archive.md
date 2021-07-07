# Archive

Types that implement `Archive` have an alternate representation that supports zero-copy
deserialization. The construction of archived types happens in two steps:

1. Any dependencies of the type are serialized. For strings this would be the characters of the
string, for boxes it would be the boxed value, and for vectors it would be any contained elements.
Any bookkeeping from this step is bundled into a `Resolver` type and held onto for later. This is
the *serialize* step.
2. The resolver and original value are used to construct the archived value in the output buffer.
For strings the resolver would be the position of the characters, for boxes it would be the position
of the boxed value, and for vectors it would be the position of the archived elements. With the
original values and resolvers combined, the archived version can be constructed. This is the
*resolve* step.

## Resolvers

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

When archiving, we might be tempted to serialize and resolve the first string, then serialize and
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
