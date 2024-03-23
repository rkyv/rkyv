use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Data, DeriveInput, Error, Field, Fields};

use crate::{attributes::Attributes, repr::Repr};

pub fn derive(mut input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = Attributes::parse(&input)?;
    let rkyv_path = attributes.rkyv_path();

    let where_clause = input.generics.make_where_clause();

    let repr = Repr::from_attrs(&input.attrs)?;

    match &input.data {
        Data::Struct(_) => {
            if !repr.is_struct_well_defined() {
                return Err(Error::new_spanned(
                    &input.ident,
                    "struct must be `repr(C)` or `repr(transparent)` to \
                     implement `Portable`",
                ));
            }
        }
        Data::Enum(_) => {
            if !repr.is_enum_well_defined() {
                return Err(Error::new_spanned(
                    &input.ident,
                    "enum must be `repr(u8/i8)` or `repr(C, u8/i8)` to \
                     implement `Portable`",
                ));
            }
        }
        Data::Union(_) => {
            if !repr.is_struct_well_defined() {
                return Err(Error::new_spanned(
                    &input.ident,
                    "union must be `repr(C)` or `repr(transparent)` to \
                     implement `Portable`",
                ));
            }
        }
    }

    iter_fields(&input.data, |f| {
        let ty = &f.ty;
        where_clause.predicates.push(parse_quote! {
            #ty: #rkyv_path::Portable
        });
    });

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();

    Ok(quote! {
        unsafe impl #impl_generics #rkyv_path::Portable for #name #ty_generics #where_clause {}
    })
}

fn iter_fields_inner(fields: &Fields, f: impl FnMut(&Field)) {
    match fields {
        Fields::Named(fields) => fields.named.iter().for_each(f),
        Fields::Unnamed(fields) => fields.unnamed.iter().for_each(f),
        Fields::Unit => (),
    }
}

fn iter_fields(data: &Data, mut f: impl FnMut(&Field)) {
    match data {
        Data::Struct(data) => iter_fields_inner(&data.fields, f),
        Data::Enum(data) => {
            for variant in &data.variants {
                iter_fields_inner(&variant.fields, &mut f);
            }
        }
        Data::Union(data) => data.fields.named.iter().for_each(f),
    }
}
