//! Procedural macros for `rkyv_typename`.

#![deny(
    rustdoc::broken_intra_doc_links,
    missing_docs,
    rustdoc::missing_crate_level_docs
)]

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, AttrStyle, DeriveInput, Error, Expr, ExprLit,
    GenericParam, Lit, Meta,
};

#[derive(Default)]
struct Attributes {
    typename: Option<String>,
}

fn parse_attributes(input: &DeriveInput) -> Result<Attributes, TokenStream> {
    let mut result = Attributes::default();

    for attr in input.attrs.iter() {
        let AttrStyle::Outer = attr.style else {
            continue;
        };

        let Meta::NameValue(ref meta) = attr.meta else {
            continue;
        };

        if !meta.path.is_ident("typename") {
            continue;
        }

        if result.typename.is_some() {
            let msg = "typename attribute already specified";

            return Err(Error::new_spanned(meta, msg).to_compile_error());
        }

        if let Expr::Lit(ExprLit {
            lit: Lit::Str(ref lit_str),
            ..
        }) = meta.value
        {
            result.typename = Some(lit_str.value());
        } else {
            let msg = "typename must be set to a string";

            return Err(Error::new_spanned(&meta.value, msg).to_compile_error());
        }
    }

    Ok(result)
}

/// Derives `TypeName` for the labeled type.
///
/// A custom name can be set using the attribute `#[typename = "..."]`.
#[proc_macro_derive(TypeName, attributes(typename))]
pub fn type_name_derive(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name_impl = derive_type_name_impl(&input);

    proc_macro::TokenStream::from(type_name_impl)
}

fn derive_type_name_impl(input: &DeriveInput) -> TokenStream {
    let attributes = match parse_attributes(input) {
        Ok(result) => result,
        Err(error) => return error,
    };

    let typename_where_predicates = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote! { #name: rkyv_typename::TypeName }
    });

    let name = &input.ident;
    let module_path = if attributes.typename.is_none() {
        quote! {
            f(core::concat!(core::module_path!(), "::"));
        }
    } else {
        quote! {}
    };
    let name_str = attributes
        .typename
        .unwrap_or_else(|| input.ident.to_string());

    let mut generics = input.generics.params.iter().filter_map(|p| {
        match p {
            GenericParam::Type(t) => {
                let name = &t.ident;
                Some(quote! { #name::build_type_name(&mut f) })
            }
            GenericParam::Const(c) => {
                let value = &c.ident;
                Some(quote! {
                    // This works for all const generic types that are supported by rust 1.68 or
                    // below. It happens to be, for these types, that the Debug trait is
                    // implemented in such a way that it generates the correct output for the
                    // expected behavior.
                    // However newer versions of the compiler may break this code in subtle and/or
                    // less subtle ways.
                    let const_val = &format!("{:?}", #value);
                    f(const_val);
                })
            }
            GenericParam::Lifetime(_) => None,
        }
    });
    let build_args = if let Some(first) = generics.next() {
        let name_str = format!("{}<", name_str);
        quote! {
            #module_path
            f(#name_str);
            #first;
            #(f(", "); #generics;)*
            f(">");
        }
    } else {
        quote! {
            #module_path
            f(#name_str)
        }
    };

    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();
    let standard_derive_where_predicates = where_clause.map(|w| &w.predicates);
    quote! {
        const _: () = {
            use rkyv_typename::TypeName;

            impl #impl_generics TypeName for #name #ty_generics
            where
                #(#typename_where_predicates,)*
                #standard_derive_where_predicates
            {
                fn build_type_name<TYPENAME__F: FnMut(&str)>(mut f: TYPENAME__F) {
                    #build_args
                }
            }
        };
    }
}
