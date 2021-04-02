# Unsized Types

rkyv supports unsized types out of the box and ships with implementations for the most common
unsized types (`str`s and slices). Trait objects can also be supported with `rkyv_dyn`, see
["Trait Objects"](trait_objects.html) for more details.

## Metadata

The core concept that enables unsized types is metadata. In rust, pointers to types can be different
sizes, in contrast with languages like C and C++ where all pointers are the same size. This is
important for the concept of sizing, which you may have encountered through rust's
[Sized](https://doc.rust-lang.org/std/marker/trait.Sized.html) trait.

Pointers are composed of two pieces: a data address and some metadata. The data address is what most
people think of when they think about pointers; it's the location of the pointed data. The metadata
for a pointer is some extra data that is needed to work safely with the data at the pointed
location. It can be almost anything, or nothing at all (for Sized types). Pointers with no extra
metadata are sometimes called "thin" pointers, and pointers _with_ metadata are sometimes called
"fat" pointers.

Fundamentally, the metadata of a pointer exists to provide the program enough information to safely
access, drop, and deallocate structures that are pointed to. For slices, the metadata carries the
length of the slice, for trait objects it carries the virtual function table (vtable) pointer, and
for custom unsized structs it carries the metadata of the single trailing unsized member.

## Archived Metadata

For unsized types, the metadata for a type is archived separately from the relative pointer to the
data. This mirrors how rust works internally to support archiving shared pointers and other exotic
use cases. This does complicate things somewhat, but for most people the metadata archiving process
will end up as just filling out a few functions and returning `()`.