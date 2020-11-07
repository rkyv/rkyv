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
    Index,
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

    let generic_params = input.generics.params.iter().map(|p| quote! { #p });
    let generic_params = quote! { #(#generic_params,)* };

    let generic_args = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { p.ident.span() => #name }
    });
    let generic_args = quote! { #(#generic_args,)* };

    let generic_predicates = match input.generics.where_clause {
        Some(ref clause) => {
            let predicates = clause.predicates.iter().map(|p| quote! { #p });
            quote! { #(#predicates,)* }
        },
        None => quote! {},
    };

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
                        quote_spanned! { f.span() => #name: self.#name.resolve(pos + offset_of!(Archived<#generic_args>, #name), &value.#name) }
                    });

                    quote! {
                        struct Resolver<#generic_params>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            #(#resolver_fields,)*
                        }

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;

                            fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                                Self::Archived {
                                    #(#archived_values,)*
                                }
                            }
                        }

                        struct Archived<#generic_params>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            #(#archived_fields,)*
                        }

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;
                            type Resolver = Resolver<#generic_args>;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Self::Resolver {
                                    #(#resolver_values,)*
                                })
                            }
                        }
                    }
                },
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: Archive }
                    });
                    let field_wheres = quote! { #(#field_wheres,)* };

                    let resolver_fields = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => archive::Resolver<#ty> }
                    });

                    let resolver_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let index = Index::from(i);
                        quote_spanned! { f.span() => self.#index.archive(writer)? }
                    });

                    let archived_fields = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => archive::Archived<#ty> }
                    });

                    let archived_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let index = Index::from(i);
                        quote_spanned! { f.span() => self.#index.resolve(pos + offset_of!(Archived<#generic_args>, #index), &value.#index) }
                    });

                    quote! {
                        struct Resolver<#generic_params>(#(#resolver_fields,)*)
                        where
                            #generic_predicates
                            #field_wheres;

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;

                            fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                                Archived::<#generic_args>(
                                    #(#archived_values,)*
                                )
                            }
                        }

                        struct Archived<#generic_params>(#(#archived_fields,)*)
                        where
                            #generic_predicates
                            #field_wheres;

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;
                            type Resolver = Resolver<#generic_args>;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Resolver::<#generic_args>(
                                    #(#resolver_values,)*
                                ))
                            }
                        }
                    }
                },
                Fields::Unit => {
                    quote! {
                        struct Resolver;

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver
                        where
                            #generic_predicates
                        {
                            type Archived = #name<#generic_args>;

                            fn resolve(self, _pos: usize, _value: &#name<#generic_args>) -> Self::Archived {
                                #name::<#generic_args>
                            }
                        }

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                        {
                            type Archived = #name<#generic_args>;
                            type Resolver = Resolver;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Resolver)
                            }
                        }
                    }
                }
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