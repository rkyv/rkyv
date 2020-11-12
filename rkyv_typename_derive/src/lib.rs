//! Procedural macros for rkyv_typename.

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    AttrStyle,
    DeriveInput,
    Error,
    Lit,
    Meta,
    parse_macro_input,
    spanned::Spanned,
};

struct Attributes {
    typename: Option<String>,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            typename: None,
        }
    }
}

fn parse_attributes(input: &DeriveInput) -> Result<Attributes, TokenStream> {
    let mut result = Attributes::default();
    for a in input.attrs.iter() {
        if let AttrStyle::Outer = a.style {
            if let Ok(meta) = a.parse_meta() {
                if let Meta::NameValue(meta) = meta {
                    if meta.path.is_ident("typename") {
                        if result.typename.is_none() {
                            if let Lit::Str(ref lit_str) = meta.lit {
                                result.typename = Some(lit_str.value());
                            } else {
                                return Err(Error::new(meta.lit.span(), "typename must be set to a string").to_compile_error());
                            }
                        } else {
                            return Err(Error::new(meta.span(), "typename attribute already specified").to_compile_error());
                        }
                    }
                }
            }
        }
    }
    Ok(result)
}

/// Derives `TypeName` for the labeled type.
///
/// A custom name can be set using the attribute `#[typename = "..."]`.
#[proc_macro_derive(TypeName, attributes(typename))]
pub fn type_name_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name_impl = derive_type_name_impl(&input);

    proc_macro::TokenStream::from(type_name_impl)
}

fn derive_type_name_impl(input: &DeriveInput) -> TokenStream {
    let attributes = match parse_attributes(input) {
        Ok(result) => result,
        Err(error) => return error,
    };

    let generic_params = input.generics.params.iter().map(|p| quote_spanned! { p.span() => #p });
    let generic_args = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { name.span() => #name }
    });
    let generic_predicates = match input.generics.where_clause {
        Some(ref clause) => {
            let predicates = clause.predicates.iter().map(|p| quote_spanned! { p.span() => #p });
            quote! { #(#predicates,)* }
        },
        None => quote! {},
    };

    let type_wheres = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { name.span() => #name: rkyv_typename::TypeName }
    });

    let name = &input.ident;
    let name_str = attributes.typename.unwrap_or_else(|| input.ident.to_string());

    let build_args = if !input.generics.params.is_empty() {
        let mut results = input.generics.type_params().map(|p| {
            let name = &p.ident;
            quote_spanned! { name.span() => #name::build_type_name(&mut f) }
        });
        let first = results.next().unwrap();
        let name_str = format!("{}<", name_str);
        quote! {
            f(#name_str);
            #first;
            #(f(", "); #results;)*
            f(">");
        }
    } else {
        quote! {
            f(#name_str)
        }
    };

    quote! {
        const _: () = {
            use rkyv_typename::TypeName;

            impl<#(#generic_params,)*> TypeName for #name<#(#generic_args,)*>
            where
                #generic_predicates
                #(#type_wheres,)*
            {
                fn build_type_name<F: FnMut(&str)>(mut f: F) {
                    #build_args
                }
            }
        };
    }
}
