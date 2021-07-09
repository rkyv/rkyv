# Serialize

Types implement [`Serialize`](https://docs.rs/rkyv/0.7.1/rkyv/trait.Serialize.html) separately from
`Archive`. `Serialize` creates a resolver for some object, then `Archive` turns the value and that
resolver into an archived type. Having a separate `Serialize` trait is necessary because although a
type may have only one archived representation, you may have options of what requirements to meet in
order to create one.

> The `Serialize` trait is parameterized over the *serializer*. The serializer is just a mutable
> object that helps the type serialize itself. The most basic types like `u32` or `char` don't
> *bound* their serializer type because they can serialize themselves with any kind of serializer.
> More complex types like `Box` and `String` require a serializer that implements
> [`Serializer`](https://docs.rs/rkyv/0.7.1/rkyv/ser/trait.Serializer.html), and even more complex
> types like `Rc` and `Vec` require a serializer that additionally implement
> [`SharedSerializeRegistry`](https://docs.rs/rkyv/0.7.1/rkyv/ser/trait.SharedSerializeRegistry.html)
> or [`ScratchSpace`](https://docs.rs/rkyv/0.7.1/rkyv/ser/trait.ScratchSpace.html).

Unlike `Serialize`, `Archive` doesn't parameterize over the serializer used to make it. It shouldn't
matter what serializer a resolver was made with, only that it's made correctly.

## Serializer

rkyv provides serializers that provide all the functionality needed to serialize standard library
types, as well as serializers that combine other serializers into a single object with all of the
components' capabilities.

The [provided serializers](https://docs.rs/rkyv/0.7.1/rkyv/ser/serializers/index.html) offer a wide
range of strategies and capabilities, but most use cases will be best suited by
[`AllocSerializer`](https://docs.rs/rkyv/0.7.1/rkyv/ser/serializers/type.AllocSerializer.html).

> Many types require *scratch space* to serialize. This is some extra allocated space that they can
> use temporarily and return when they're done. For example, `Vec` might request scratch space to
> store the resolvers for its elements until it can serialize all of them. Requesting scratch space
> from the serializer allows scratch space to be reused many times, which reduces the number of slow
> memory allocations performed while serializing.
