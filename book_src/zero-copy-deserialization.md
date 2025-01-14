# Zero-copy deserialization

Zero-copy deserialization is a technique that reduces the time and memory required to access and use
data by *directly referencing bytes in the serialized form*.

> This takes advantage of how we have to have some data loaded in memory in order to deserialize it.
> If we had some JSON:
> ```
> { "quote": "I don't know, I didn't listen." }
>             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
> ```
> Instead of copying those characters into a `String`, we could just *borrow* it from the JSON
> buffer as a `&str`. The lifetime of that `&str` would depend on our buffer and we wouldn't be
> allowed to drop it until we had dropped the string we were using.

## Partial zero-copy

Serde and others have support for partial zero-copy deserialization, where bits and pieces of the
deserialized data are borrowed from the serialized form. Strings, for example, can borrow their
bytes directly from the serialized form in encodings like bincode that don't perform any character
escaping. However, a string object must still be created to hold the deserialized length and point
to the borrowed characters.

> A good way to think about this is that even though we're borrowing lots of data from the buffer,
> we still have to parse the *structure* out:
> ```rs
> struct Example<'a> {
>   quote: &'a str,
>   a: &'a [u8; 12],
>   b: u64,
>   c: char,
> }
> ```
> So a buffer might break down like this:
> ```
> I don't know, I didn't listen.AAAAAAAAAAAABBBBBBBBCCCC
> ^-----------------------------^-----------^-------^---
>  quote: str                    a: [u8; 12] b: u64  c: char
> ```
> We do a lot less work, but we still have to parse, create, and return an `Example<'a>`:
> ```rs
> Example {
>   quote: str::from_utf8(&buffer[0..30]).unwrap(),
>   a: &buffer[30..42],
>   b: u64::from_le_bytes(&buffer[42..50]),
>   c: char::from_u32(u32::from_le_bytes(&buffer[50..54]))).unwrap(),
> }
> ```
> And we can't borrow types like `u64` or `char` that have alignment requirements since our buffer
> might not be properly aligned. We have to immediately parse and store those! Even though we
> borrowed 42 of the buffer's bytes, we missed out on the last 12 and still had to parse through the
> buffer to find out where everything is.

Partial zero-copy deserialization can considerably improve memory usage and often speed up
some deserialization, but with some work we can go further.

## Total zero-copy

rkyv implements total zero-copy deserialization, which guarantees that no data is copied during
deserialization and no work is done to deserialize data. It achieves this by structuring its encoded
representation so that it is the same as the in-memory representation of the source type.

> This is more like if our buffer *was* an Example:
> ```rs
> struct Example {
>   quote: String,
>   a: [u8; 12],
>   b: u64,
>   c: char,
> }
> ```
> And our buffer looked like this:
> ```
> I don't know, I didn't listen.__QOFFQLENAAAAAAAAAAAABBBBBBBBCCCC
> ^-----------------------------  ^---^---^-----------^-------^---
>  quote bytes                    pointer  a           b       c
>                                 and len
>                                 ^-------------------------------
>                                  Example
> ```
> In this case, the bytes are padded to the correct alignment and the fields of `Example` are laid
> out exactly the same as they would be in memory. Our deserialization code can be much simpler:
> ```rs
> unsafe { &*buffer.as_ptr().add(32).cast() }
> ```
> This operation is almost zero work, and more importantly it doesn't *scale* with our data. No
> matter how much or how little data we have, it's always just a pointer offset and a cast to access
> our data.

This opens up blazingly-fast data loading and enables data access orders of magnitude more quickly
than traditional serialization.
