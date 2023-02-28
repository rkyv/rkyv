# Validation

Validation can be enabled with the `validation` feature. Validation leverages the
[`bytecheck`](https://docs.rs/bytecheck) crate to perform archive validation, and allows the
consumption of untrusted and malicious data.

To validate an archive, you first have to derive
[`CheckBytes`](https://docs.rs/bytecheck/latest/bytecheck/trait.CheckBytes.html) for your archived
type:

```rs
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
pub struct Example {
    a: i32,
    b: String,
    c: Vec<bool>,
}
```

The `#[archive(check_bytes)]` attribute derives `CheckBytes` on the archived type. Finally, you can use
[`check_archived_root`](https://docs.rs/rkyv/0.7.1/rkyv/validation/validators/fn.check_archived_root.html) to
check an archive and get a reference to the archived value if it was successful:

```rs
use rkyv::check_archived_root;

let archived_example = check_archived_root::<Example>(buffer).unwrap();
```

More examples of how to enable and perform validation can be found in the `rkyv_test` crate's
`validation` module.

## The validation context

When checking an archive, a validation context is created automatically using some good defaults
that will work for most archived types. If your type requires special validation logic, you may need
to augment the capabilities of the validation context in order to check your type and use
[`check_archived_root_with_context`](https://docs.rs/rkyv/0.7.1/rkyv/validation/fn.check_archived_root_with_context.html).

> The
> [`DefaultValidator`](https://docs.rs/rkyv/latest/rkyv/validation/validators/struct.DefaultValidator.html)
> supports all builtin rkyv types, but changes depending on whether you have the `alloc` feature
> enabled or not.

## Bounds checking and subtree ranges

All pointers are checked to make sure that they:

- point inside the archive
- are properly aligned
- and have enough space afterward to hold the desired object

However, this alone is not enough to secure against recursion attacks and memory sharing violations,
so rkyv uses a system to verify that the archive follows its strict ownership model.

Archive validation uses a memory model where all subobjects are located in contiguous memory. This
is called a *subtree range*. When validating an object, the archive context keeps track of where
subobjects are allowed to be located, and can reduce the subtree range from the beginning with
`push_prefix_subtree_range` or the end with `push_suffix_subtree_range`. After pushing a subtree
range, any subobjects in that range can be checked by calling their `CheckBytes` implementations.
Once the subobjects are checked, `pop_prefix_subtree_range` and `pop_suffix_subtree_range` can be
used to restore the original range with the checked section removed.

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
