use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Data, DeriveInput, Error};

use crate::{attributes::Attributes, repr::Repr, util::iter_fields};

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
