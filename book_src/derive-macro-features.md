# Derive macro features

rkyv's derive macro supports a number of attributes and configurable options. All of rkyv's macro
attributes are documented on the `Archive` proc-macro. Some of the most important ones to know are:

## `omit_bounds`

rkyv's derive macro performs a "perfect derive" by default. This means that when it generates trait
impls, it adds where clauses requiring each field type to also implement that trait. This can cause
trouble in two primary situations:

1. Recursive type definitions (using e.g. `Box`) cause an overflow and never finish evaluating
2. Private types may be exposed by these derive bounds.

Both of these situations can be fixed by adding `#[rkyv(omit_bounds)]` on the field. This prevents
rkyv from adding the "perfect derive" bounds for that field.

When you do omit the bounds for a particular field, it can lead to insufficient bounds being added
to the generated impl. To add custom bounds back, you can use:

- `#[rkyv(archive_bounds(..))]` to add predicates to all generated impls
- `#[rkyv(serialize_bounds(..))]` to add predicates to just the `Serialize` impl
- `#[rkyv(deserialize_bounds(..))]` to add predicates to just the `Deserialize` impl

See `rkyv/examples/json_like_schema.rs` for a fully-commented example of using `omit_bounds`.

## `with = ..`

This customizes the serialization of a field by applying a
[wrapper type](derive-macro-features/wrapper-types.md).

## `remote = ..`

This performs a [remote derive](derive-macro-features/remote-derive.md) for supporting external
types.

## `attr(..)` and `derive(..)`

`#[rkyv(attr(..))]` is a general-purpose attribute which allows you to pass attributes down to the
generated archived type. This can be especially useful in combination with `#[rkyv(derive(..))]`,
which may be used on types and is sugar for `#[rkyv(attr(derive(..)))]`.
