use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, DeriveInput, Error};

use crate::{attributes::Attributes, util::iter_fields};

pub fn derive(mut input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = Attributes::parse(&input)?;
    let rkyv_path = attributes.crate_path();

    let where_clause = input.generics.make_where_clause();

    for field in iter_fields(&input.data) {
        let ty = &field.ty;
        where_clause.predicates.push(parse_quote! {
            #ty: #rkyv_path::traits::Freeze
        });
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();

    Ok(quote! {
        unsafe impl #impl_generics #rkyv_path::traits::Freeze
            for #name #ty_generics
        #where_clause
        {}
    })
}
