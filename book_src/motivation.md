# Motivation

First and foremost, the motivation behind rkyv is improved performance. The way that it achieves
that goal can also lead to gains in memory use, correctness, and security along the way.

> Familiarity with other serialization frameworks and how traditional serialization works will help,
> but isn't necessary to understand how rkyv works.

Most serialization frameworks like [serde](https://serde.rs) define an internal data model that
consists of basic types such as primitives, strings, and byte arrays. This splits the work of
serializing a type into two stages: the frontend and the backend. The frontend takes some type and
breaks it down into the serializable types of the data model. The backend then takes the data model
types and writes them using some data format such as JSON, Bincode, TOML, etc. This allows a clean
separation between the serialization of a type and the data format it is written to.

> Serde describes [its data model](https://serde.rs/data-model.html) in the serde book. Everything
> serialized with serde eventually boils down to some combination of those types!

A major downside of traditional serialization is that it takes a considerable amount of time to
read, parse, and reconstruct types from their serialized values.

> In JSON for example, strings are encoded by surrounding the contents with double quotes and
> escaping invalid characters inside of them:
> ```
> { "line": "\"All's well that ends well\"" }
>           ^^                          ^ ^
> ```
> numbers are turned into characters:
> ```
> { "pi": 3.1415926 }
>         ^^^^^^^^^
> ```
> and even field names, which could be *implicit* in most cases, are turned into strings:
> ```
> { "message_size": 334 }
>   ^^^^^^^^^^^^^^^
> ```
> All those characters are not only taking up space, they're also taking up time. Every time we read
> and parse JSON, we're picking through those characters in order to figure out what the values are
> and reproduce them in memory. An `f32` is only four bytes of memory, but it's encoded using nine
> bytes and we still have to turn those nine characters into the right `f32`!

This deserialization time adds up quickly, and in data-heavy applications such as games and media
editing it can come to dominate load times. rkyv provides a solution through a serialization
technique called *zero-copy deserialization*.
