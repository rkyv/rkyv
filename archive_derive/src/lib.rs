extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    Data,
    DeriveInput,
    Fields,
    parse_macro_input,
    spanned::Spanned,
};

#[proc_macro_derive(Archive)]
pub fn archive_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let archive_impl = derive_archive_impl(&input);

    proc_macro::TokenStream::from(archive_impl)
}

fn derive_archive_impl(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;

    let archive_impl = match input.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: Archive }
                    });
                    let field_wheres = quote! { #(#field_wheres,)* };

                    let resolver_fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #name: archive::Resolver<#ty> }
                    });

                    let resolver_values = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! { f.span() => #name: self.#name.archive(writer)? }
                    });

                    let archived_fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #name: archive::Archived<#ty> }
                    });

                    let archived_values = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! { f.span() => #name: self.#name.resolve(pos + offset_of!(Archived, #name), &value.#name) }
                    });

                    quote! {
                        struct Resolver
                        where
                            #field_wheres
                        {
                            #(#resolver_fields,)*
                        }

                        impl Resolve<#name> for Resolver {
                            type Archived = Archived;

                            fn resolve(self, pos: usize, value: &#name) -> Self::Archived {
                                Self::Archived {
                                    #(#archived_values,)*
                                }
                            }
                        }

                        struct Archived
                        where
                            #field_wheres
                        {
                            #(#archived_fields,)*
                        }

                        impl Archive for #name
                        where
                            #field_wheres
                        {
                            type Archived = Archived;
                            type Resolver = Resolver;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Self::Resolver {
                                    #(#resolver_values,)*
                                })
                            }
                        }
                    }
                },
                _ => quote! {},
            }
        },
        _ => quote! {},
    };

    quote! {
        const _: () = {
            use archive::{
                Archive,
                offset_of,
                Resolve,
                Write,
            };
            #archive_impl
        };
    }
}