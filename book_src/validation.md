# Validation

Validation can be enabled with the `validation` feature. Validation leverages the
[`bytecheck`](https://docs.rs/bytecheck) crate to perform archive validation, and allows the
consumption of untrusted and malicious data.

To validate an archive, use
[`check_archived_root`](https://docs.rs/rkyv/latest/rkyv/validation/fn.check_archived_root.html). Examples of
how to enable and perform validation can be found in the `rkyv_test` crate's `validation` module.

## Validation and Shared Pointers

While validating shared pointers is supported, some additional restrictions are in place to prevent
malicious data from validating:

Shared pointers that point to the same object will fail to validate if they are different types.
This can cause issues if you have a shared pointer to the same array, but the pointers are an array
pointer and a slice pointer. Similarly, it can cause issues if you have shared pointers to the same
value as a concrete type (e.g. `i32`) and a trait object (e.g. `dyn Any`).

rkyv still supports these use cases, but it's not possible or feasible to ensure data integrity with
these use cases. Alternative validation solutions like archive signatures and data hashes may be a
better approach in these cases.
