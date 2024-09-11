<p align="center">
    <img src="https://raw.githubusercontent.com/rkyv/rkyv/master/media/logo_text_color.svg">
</p>

[rkyv](http://github.com/rkyv/rkyv) (*archive*) is a zero-copy deserialization framework for
Rust.

This book covers the motivation, architecture, and major features of rkyv. It is the best way to
learn and understand rkyv, but won't go as in-depth on specifics as the documentation will. Don't be
afraid to consult these other resources as you need while you read through.

# Resources

## Learning Materials

- The [rkyv discord](https://discord.gg/65F6MdnbQh) is a great place to get help with specific
  issues and meet other people using rkyv
- The [rkyv github](https://github.com/rkyv/rkyv) hosts the source and tracks project issues
  and milestones.

## Documentation

- [rkyv](https://docs.rs/rkyv), the core library
- [rkyv_dyn](https://docs.rs/rkyv_dyn), which adds trait object support to rkyv

## Benchmarks

- The [rust serialization benchmark](https://github.com/djkoloski/rust_serialization_benchmark) is a
  shootout style benchmark comparing many rust serialization solutions. It includes special
  benchmarks for zero-copy serialization solutions like rkyv.

## Sister Crates

- [rend](https://github.com/rkyv/rend), which rkyv uses for endian-agnostic features
- [bytecheck](https://github.com/rkyv/bytecheck), which rkyv uses for validation
- [rancor](https://github.com/rkyv/rancor), which rkyv uses for error handling
- [ptr_meta](https://github.com/rkyv/ptr_meta), which rkyv uses for pointer manipulation
