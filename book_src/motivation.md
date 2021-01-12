# Motivation

Most serialization frameworks like [serde](https://serde.rs) define an internal data model that
consists of basic types such as primitives, strings, and byte arrays. This splits the work of
serializing a type into two stages: the frontend and the backend. The frontend takes some type and
breaks it down into the serializable types of the data model. The backend then takes the data model
types and writes them using some data format such as JSON, Bincode, TOML, etc. This allows a clean
separation between the serialization of a type and the data format it is written to.

A major downside of traditional serialization is that it takes a considerable amount of time to
read, parse, and reconstruct types from their serialized values. In JSON, for example, strings are
encoded by surrounding the contents with double quotes and escaping invalid characters inside of
them. Deserializing these strings entails parsing character-by-character for double quotes and
escape characters, and pushing the parsed characters into a result string. This deserialization time
adds up quickly, and in data-heavy applications such as games and media editing it can come to
dominate load times. rkyv provides a solution through a serialization technique called zero-copy
deserialization.