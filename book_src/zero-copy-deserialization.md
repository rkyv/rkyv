# Zero-copy deserialization

Zero-copy deserialization is a technique that reduces the time and memory required to deserialize
data by directly referencing bytes in the serialized form.

## Partial zero-copy

Serde and others have support for partial zero-copy deserialization, where bits and pieces of the
deserialized data are borrowed from the serialized form. Strings, for example, can borrow their
bytes directly from the serialized form in encodings like bincode that don't perform any character
escaping. However, a string object must still be created to hold the deserialized length and point
to the borrowed characters.

Partial zero-copy deserialization can considerably improve memory usage and often speed up
some deserialiation, but with some work we can go further.

## Total zero-copy

rkyv implements total zero-copy deserialization, which guarantees that no data is copied during
deserialization and no work is done to deserialize data. It achieves this by structuring its encoded
representation so that it is the same as the in-memory representation of the source type.

This opens up blazingly-fast data loading and enables data access orders of magnitude more quickly
than traditional serialization.