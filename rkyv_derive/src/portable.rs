use proc_macro2::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_quote, Data, DeriveInput, Error, Path};

use crate::{attributes::Attributes, repr::Repr, util::iter_fields};

/// Checks that `crate_path` names the actual `rkyv` crate.
///
/// `derive(Portable)` emits an `unsafe impl`, so the path it names must not be
/// spoofable: a safe derive call must not be able to implement another crate's
/// `Portable` trait.
fn check_crate_path(crate_path: &Path) -> Result<(), Error> {
    let names_rkyv = |path: &Path, name: &str| {
        path.leading_colon.is_some()
            && path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == name)
    };

    match crate_name("rkyv") {
        // Inside `rkyv` itself the crate is reachable as `crate` for items
        // and as `::rkyv` for doctests and examples.
        Ok(FoundCrate::Itself) => {
            if crate_path.is_ident("crate") || names_rkyv(crate_path, "rkyv") {
                Ok(())
            } else {
                Err(Error::new_spanned(
                    crate_path,
                    "`derive(Portable)` can only implement the `rkyv` trait; \
                     inside `rkyv`, use `crate` or `::rkyv`",
                ))
            }
        }
        Ok(FoundCrate::Name(name)) => {
            if names_rkyv(crate_path, &name) {
                Ok(())
            } else {
                Err(Error::new_spanned(
                    crate_path,
                    format!(
                        "`derive(Portable)` can only implement the `rkyv` \
                         trait; use the absolute path `::{name}` of this \
                         crate's `rkyv` dependency"
                    ),
                ))
            }
        }
        Err(_) => Err(Error::new_spanned(
            crate_path,
            "`derive(Portable)` requires `rkyv` to be a direct dependency of \
             this crate",
        )),
    }
}

pub fn derive(mut input: DeriveInput) -> Result<TokenStream, Error> {
    let repr = Repr::from_attrs(&input.attrs)?;
    match &input.data {
        Data::Struct(_) | Data::Union(_) => {
            if !repr.is_struct_well_defined() {
                return Err(Error::new_spanned(
                    &input.ident,
                    "structs and unions must be `repr(C)` or \
                     `repr(transparent)` to implement `Portable`",
                ));
            }
        }
        Data::Enum(_) => {
            if !repr.is_enum_well_defined() {
                return Err(Error::new_spanned(
                    &input.ident,
                    "enums must be `repr(u8/i8)` or `repr(C, u8/i8)` to \
                     implement `Portable`",
                ));
            }
        }
    }

    let attributes = Attributes::parse(&input)?;
    let rkyv_path = attributes.crate_path();
    check_crate_path(&rkyv_path)?;

    let where_clause = input.generics.make_where_clause();

    for field in iter_fields(&input.data) {
        let ty = &field.ty;
        where_clause.predicates.push(parse_quote! {
            #ty: #rkyv_path::Portable
        });
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();

    Ok(quote! {
        unsafe impl #impl_generics #rkyv_path::Portable for #name #ty_generics
        #where_clause
        {}
    })
}
