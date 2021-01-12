# Validation

Validation can be enabled with the `validation` feature. Validation leverages the
[`bytecheck`](https://docs.rs/bytecheck) crate to perform archive validation, and allows the
consumption of untrusted and malicious data.

To validate an archive, use
[`check_archive`](https://docs.rs/rkyv/latest/rkyv/validation/fn.check_archive.html). Examples of
how to enable and perform validation can be found in the `rkyv_test` crate's `validation` module.