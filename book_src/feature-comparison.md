# Feature Comparison

This is a best-effort feature comparison between rkyv, FlatBuffers, and Cap'n Proto. This is by no
means completely comprehensive, and pull requests that improve this are welcomed.

## Feature matrix

| Feature                           | rkyv          | Cap'n Proto   | FlatBuffers   |
|-----------------------------------|---------------|---------------|---------------|
| Open type system                  | yes           | no            | no            |
| Scalars                           | yes           | no            | yes           |
| Tables                            | no            | yes           | yes           |
| Schema evolution                  | no            | yes           | yes           |
| Zero-copy                         | yes           | yes           | yes           |
| Random-access reads               | yes           | yes           | yes           |
| Validation                        | upfront       | on-demand     | yes           |
| Reflection                        | no            | yes           | yes           |
| Object order                      | bottom-up     | either        | bottom-up     |
| Schema language                   | derive        | custom        | custom        |
| Usable as mutable state           | limited       | limited       | limited       |
| Padding takes space on wire?      | optional      | optional      | no            |
| Unset fields take space on wire?  | yes           | yes           | no            |
| Pointers take space on wire?      | yes           | yes           | yes           |
| Cross-language                    | no            | yes           | yes           |
| Hash maps and B-trees             | yes           | no            | no            |
| Shared pointers                   | yes           | no            | no            |

Although these features aren't supported out-of-the-box, rkyv's open type system allows extensions
which provide many of these capabilities.

## Open type system

One of rkyv's primary features is that its type system is *open*. This means that users can write
custom types and control their properties very finely. You can think of rkyv as a solid foundation
to build many other features on top of. In fact, the open type system is already a fundamental part
of how rkyv works.

### Unsized types

Even though they're part of the main library, unsized types are built on top of the core
serialization functionality. Types like `Box` and `Rc/Arc` that can hold unsized types are entry
points for unsized types into the sized system.

### Trait objects

Trait objects are further built on top of unsized types to make serializing and using trait objects
easy and safe.
