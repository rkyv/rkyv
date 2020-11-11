extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    DeriveInput,
    parse_macro_input,
    spanned::Spanned,
};

#[proc_macro_derive(TypeName)]
pub fn type_name_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name_impl = derive_type_name_impl(&input);

    proc_macro::TokenStream::from(type_name_impl)
}

fn derive_type_name_impl(input: &DeriveInput) -> TokenStream {
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
        quote_spanned! { name.span() => #name: type_name::TypeName }
    });

    let name = &input.ident;

    let build_args = if input.generics.params.len() > 0 {
        let mut results = input.generics.type_params().map(|p| {
            let name = &p.ident;
            quote_spanned! { name.span() => #name::build_type_name(&mut f) }
        });
        let first = results.next().unwrap();
        let name_str = format!("{}<", input.ident);
        quote! {
            f(#name_str);
            #first;
            #(f(", "); #results;)*
            f(">");
        }
    } else {
        let name_str = input.ident.to_string();

        quote! {
            f(#name_str)
        }
    };

    quote! {
        const _: () = {
            use type_name::TypeName;

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
