# Shared Pointers

The implementation details of shared pointers may be of interest to those using them. The rules
surrounding how and when shared and weak pointers are serialized and pooled may affect how you
choose to use them.

## Serialization

Shared pointers (`Rc` and `Arc`) are serialized whenever they're encountered for the first time, and
the data address is reused when subsequent shared pointers point to the same data. This means that
you can expect shared pointers to always point to the same value when archived, even if they are
unsized to different types.

Weak pointers (`rc::Weak` and `sync::Weak`) have serialization attempted as soon as they're
encountered. The serialization process upgrades them, and if it succeeds it serializes them like
shared pointers. Otherwise, it serializes them like `None`.

## Deserialization

Similarly, shared pointers are deserialized on the first encounter and reused afterward. Weak
pointers do a similar upgrade attempt when they're encountered for the first time.

## Serializers and Deserializers

The serializers for shared pointers hold the location of the serialized data. This means it's safe
to serialize shared pointers to an archive across multiple `serialize` calls as long as you use the
same serializer for each one. Using a new serializer will still do the right thing, but may end up
duplicating the shared data.

The deserializers for shared pointers hold a shared pointer to any deserialized values, and will
hold them in memory until the deserializer is dropped. This means that if you serialize only weak
pointers to some shared data, they will point to the correct value when deserialized but will point
to nothing as soon as the deserializer is dropped.
