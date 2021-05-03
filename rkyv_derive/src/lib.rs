//! Procedural macros for `rkyv`.

mod archive;
mod attributes;
mod deserialize;
mod serialize;

extern crate proc_macro;

use syn::{parse_macro_input, DeriveInput};

/// Derives `Archive` for the labeled type.
///
/// # Attributes
///
/// Additional arguments can be specified using the `#[archive(...)]` and `#[archive_attr(...)]`
/// attributes.
///
/// `#[archive(...)]` takes the following arguments:
///
/// - `copy`: Implements `ArchiveCopy` as well as `Archive`. Only suitable for types that can be
///   directly archived (i.e. plain data).
/// - `compare(...)`: Implements common comparison operators between the original and archived
///   types. Supported comparisons are `PartialEq` and `PartialOrd` (i.e.
///   `#[archive(compare(PartialEq, PartialOrd))]`).
/// - `name`, `name = "..."`: Exposes the archived type with the given name. If used without a name
///   assignment, uses the name `"Archived" + name`.
/// - `strict`: Marks structs at `#[repr(C)]` for strictly guaranteed stability and compatibility.
///   This is equivalent to enabling the `strict` feature for only this struct.
/// - `bound(...)`: Adds additional bounds to the `Serialize` and `Deserialize` implementations.
///   This can be especially useful when dealing with recursive structures, where bounds may need to
///   be omitted to prevent recursive type definitions.
///
/// `#[archive_attr(...)]` adds the attributes passed as arguments as attributes to the generated
/// type. This is commonly used with attributes like `derive(...)` to derive trait implementations
/// for the archived type.
///
/// # Recursive types
///
/// This derive macro automatically adds a type bound `field: Archive` for each field type. This can
/// cause an overflow while evaluating trait bounds if the structure eventually references its own
/// type, as the implementation of `Archive` for a struct depends on each field type implementing it
/// as well. Adding the attribute `#[omit_bounds]` to a field will suppress this trait bound and
/// allow recursive structures. This may be too coarse for some types, in which case additional type
/// bounds may be required with `bound(...)`.
#[proc_macro_derive(Archive, attributes(archive, archive_attr, omit_bounds))]
pub fn derive_archive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match archive::derive(parse_macro_input!(input as DeriveInput)) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derives `Serialize` for the labeled type.
///
/// This macro also supports the `#[archive]` and `#[omit_bounds]` attributes. See [`Archive`] for
/// more information.
#[proc_macro_derive(Serialize, attributes(archive, omit_bounds))]
pub fn derive_serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match serialize::derive(parse_macro_input!(input as DeriveInput)) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derives `Deserialize` for the labeled type.
///
/// This macro also supports the `#[archive]` and `#[omit_bounds]` attributes. See [`Archive`] for
/// more information.
#[proc_macro_derive(Deserialize, attributes(archive, omit_bounds))]
pub fn derive_deserialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match deserialize::derive(parse_macro_input!(input as DeriveInput)) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
