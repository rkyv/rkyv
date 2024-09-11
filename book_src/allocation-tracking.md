# Allocation tracking

rkyv's provided `AllocationTracker` struct wraps an `Allocator` and tracks when memory is allocated
and freed during serialization. It can also calculate synthetic metrics, like the minimum amount of
pre-allocated memory required to serialize a value. And, it can report the maximum alignment of all
serialized types.

You can create a custom serializer with allocation tracking by calling `Serializer::new(..)` and
providing the pieces of your serializer. Normally, the provided allocator would be an `ArenaHandle`,
but instead you should provide it an `AllocationTracker::new(arena_handle)`.

After serializing your value, the serializer can be decomposed with `into_raw_parts`. You can then
retrieve the `AllocationStats` from the allocator by calling `into_stats`.
