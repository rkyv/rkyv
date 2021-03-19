//! Procedural macros for `rkyv`.

mod archive;
mod attributes;
mod deserialize;
mod serialize;

extern crate proc_macro;

use syn::{parse_macro_input, DeriveInput};

/// Derives `Archive` for the labeled type.
///
/// Additional arguments can be specified using the `#[archive(...)]` attribute:
///
/// - `copy`: Implements `ArchiveCopy` as well as `Archive`. Only suitable for
/// types that can be directly archived.
/// - `derive(...)`: Adds a `#[derive(...)]` attribute to the archived type.
/// - `name`, `name = "..."`: Exposes the archived type with the given name. If
/// used without a name assignment, uses the name `"Archived" + name`.
/// - `strict`: Marks structs at `#[repr(C)]` for strictly guaranteed stability
///   and compatibility. This is equivalent to enabling the `strict` feature for
///   only this struct.
///
/// This derive macro automatically adds a type bound `field: Archive` for each
/// field type. This can cause an overflow while evaluating trait bounds if the
/// structure eventually references its own type, as the implementation of
/// `Archive` for a struct depends on each field type implementing it as well.
/// Adding the attribute `#[recursive]` to a field will suppress this trait
/// bound and allow recursive structures. This may be too coarse for some types,
/// in which case `Archive` will have to be implemented manually.
#[proc_macro_derive(Archive, attributes(archive, recursive))]
pub fn derive_archive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match archive::derive(parse_macro_input!(input as DeriveInput)) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derives `Serialize` for the labeled type.
///
/// This macro also supports the `#[archive]` and `#[recursive]` attributes. See
/// [`Archive`] for more information.
#[proc_macro_derive(Serialize, attributes(archive, recursive))]
pub fn derive_serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match serialize::derive(parse_macro_input!(input as DeriveInput)) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derives `Deserialize` for the labeled type.
///
/// This macro also supports the `#[archive]` and `#[recursive]` attributes. See
/// [`Archive`] for more information.
#[proc_macro_derive(Deserialize, attributes(archive, recursive))]
pub fn derive_deserialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match deserialize::derive(parse_macro_input!(input as DeriveInput)) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
