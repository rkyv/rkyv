# Validation

Validation can be enabled with the `bytecheck` feature, and leverages the
[`bytecheck`](https://docs.rs/bytecheck) crate to perform archive validation. This allows the
use of untrusted and malicious data.

If the `bytecheck` feature is enabled, then rkyv will automatically derive
[`CheckBytes`](https://docs.rs/bytecheck/latest/bytecheck/trait.CheckBytes.html) for your archived
type:

```rs
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize)]
pub struct Example {
    a: i32,
    b: String,
    c: Vec<bool>,
}
```

The `#[rkyv(bytecheck(..))]` attribute passes its arguments through to the underlying `CheckBytes`
derive on the archived type. Finally, you can use `access` to check an archive and get a reference
to the archived value if it was successful:

```rs
use rkyv::{access, rancor::Failure};

let archived_example = access::<ArchivedExample, Failure>(buffer).unwrap();
```

## The validation context

When checking an archive, a validation context is created automatically using some good defaults
that will work for most archived types. If your type requires special validation logic, you may need
to augment the capabilities of the validation context in order to check your type.

## Bounds checking and subtree ranges

All pointers are checked to make sure that they:

- Point inside the archive
- Are properly aligned
- And have enough space afterward to hold the desired object

However, this alone is not enough to secure against recursion attacks and memory sharing violations,
so rkyv uses a system to verify that the archive follows its strict ownership model.

Archive validation uses a memory model where all subobjects are located in contiguous memory. This
is called a *subtree range*. When validating an object, the archive context keeps track of where
subobjects are allowed to be located, and can reduce the subtree range from the beginning by pushing
a new subtree range. After pushing a subtree range, any subobjects in that range can be checked by
calling their `CheckBytes` implementations. Once the subobjects are checked, the subtree range can
be popped to restore the original range with the checked section removed.

## Validation and Shared Pointers

While validating shared pointers is supported, some additional restrictions are in place to prevent
malicious data from validating.

Shared pointers that point to the same object will fail to validate if they are different types.
This can cause issues if you have a shared pointer to the same array, but the pointers are an array
pointer and a slice pointer. Similarly, it can cause issues if you have shared pointers to the same
value as a concrete type (e.g. `i32`) and a trait object (e.g. `dyn Any`).

rkyv still supports these use cases, but it's not possible or feasible to ensure data integrity with
these use cases. Alternative validation solutions like archive signatures and data hashes may be a
better approach in these cases.
