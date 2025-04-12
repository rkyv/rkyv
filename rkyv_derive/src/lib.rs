//! Procedural macros for `rkyv`.

#![deny(
    rustdoc::broken_intra_doc_links,
    missing_docs,
    rustdoc::missing_crate_level_docs
)]

mod archive;
mod attributes;
mod deserialize;
mod portable;
mod repr;
mod serde;
mod serialize;
mod util;

extern crate proc_macro;

use syn::{parse_macro_input, DeriveInput};

/// Derives `Portable` for the labeled type.
#[proc_macro_derive(Portable, attributes(rkyv))]
pub fn derive_portable(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut derive_input = parse_macro_input!(input as DeriveInput);
    serde::receiver::replace_receiver(&mut derive_input);

    match portable::derive(derive_input) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derives `Archive` for the labeled type.
///
/// # Attributes
///
/// Additional arguments can be specified using `#[rkyv(..)]`, which accepts
/// the following arguments:
///
/// ## Types and fields
///
/// - `attr(..)`: Passes along attributes to the generated archived type.
///
/// ## Types only
///
/// - `derive(..)`: Adds the derives passed as arguments to the generated type.
///   This is equivalent to `#[rkyv(attr(derive(..)))]`.
/// - `crate = ..`: Chooses an alternative crate path to import rkyv from.
/// - `compare(..)`: Implements common comparison operators between the original
///   and archived types. Supported comparisons are `PartialEq` and `PartialOrd`
///   (i.e. `#[rkyv(compare(PartialEq, PartialOrd))]`).
/// - `{archive, serialize, deserialize}_bounds(..)`: Adds additional bounds to
///   trait implementations. This can be useful for recursive types, where
///   bounds may need to be omitted to prevent recursive trait impls.
/// - `bytecheck(..)`: Passed through to the underlying `CheckBytes` derive for
///   the archived type.
/// - `as = ..`: Uses the given archived type instead of generating a new one.
///   This is useful for types which are `Portable` and/or generic over their
///   parameters.
/// - `archived = ..`: Changes the name of the generated archived type. By
///   default, archived types are named "Archived" + `the name of the type`.
/// - `resolver = ..`: Changes the name of the generated resolver type. By
///   default, resolver types are named `the name of the type` + "Resolver".
/// - `remote = ..`: Generate a remote derive for the annotated type instead of
///   a regular derive.
///
/// ## Fields only
///
/// - `with = ..`: Applies the given wrapper type to the field.
/// - `omit_bounds`: Omits trait bounds for the annotated field in the generated
///   impl.
/// - `niche` or `niche = ..`: Makes the outer type nichable through the
///   annotated field using the default niche or a specified one.
///
/// # Recursive types
///
/// This derive macro automatically adds a type bound `field: Archive` for each
/// field type. This can cause an overflow while evaluating trait bounds if the
/// structure eventually references its own type, as the implementation of
/// `Archive` for a struct depends on each field type implementing it
/// as well. Adding the attribute `#[rkyv(omit_bounds)]` to a field will
/// suppress this trait bound and allow recursive structures. This may be too
/// coarse for some types, in which case additional type bounds may be required
/// with `{archive, serialize, deserialize}_bounds(..)`.
///
/// # Wrappers
///
/// Wrappers transparently customize archived types by providing different
/// implementations of core traits. For example, references cannot be archived,
/// but the `Inline` wrapper serializes a reference as if it were a field of the
/// struct. Wrappers can be applied to fields using the `#[rkyv_with = ..]`
/// attribute.
#[proc_macro_derive(Archive, attributes(rkyv))]
pub fn derive_archive(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut derive_input = parse_macro_input!(input as DeriveInput);
    serde::receiver::replace_receiver(&mut derive_input);

    match archive::derive(&mut derive_input) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derives `Serialize` for the labeled type.
///
/// This macro also supports the `#[rkyv]` attribute. See [`Archive`] for more
/// information.
#[proc_macro_derive(Serialize, attributes(rkyv))]
pub fn derive_serialize(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut derive_input = parse_macro_input!(input as DeriveInput);
    serde::receiver::replace_receiver(&mut derive_input);

    match serialize::derive(derive_input) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derives `Deserialize` for the labeled type.
///
/// This macro also supports the `#[rkyv]` attribute. See [`Archive`] for more
/// information.
#[proc_macro_derive(Deserialize, attributes(rkyv))]
pub fn derive_deserialize(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut derive_input = parse_macro_input!(input as DeriveInput);
    serde::receiver::replace_receiver(&mut derive_input);

    match deserialize::derive(derive_input) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
