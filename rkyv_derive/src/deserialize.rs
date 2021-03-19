use crate::attributes::{Attributes, parse_attributes};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Error, Fields, Ident, Index, spanned::Spanned};

pub fn derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = parse_attributes(&input)?;

    if attributes.copy.is_some() {
        derive_deserialize_copy_impl(&input, &attributes)
    } else {
        derive_deserialize_impl(&input)
    }
}

fn derive_deserialize_impl(input: &DeriveInput) -> Result<TokenStream, Error> {
    let name = &input.ident;

    let generic_params = input
        .generics
        .params
        .iter()
        .map(|p| quote_spanned! { p.span() => #p });
    let generic_params = quote! { #(#generic_params,)* };

    let generic_args = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { name.span() => #name }
    });
    let generic_args = quote! { #(#generic_args,)* };

    let generic_predicates = match input.generics.where_clause {
        Some(ref clause) => {
            let predicates = clause.predicates.iter().map(|p| quote! { #p });
            quote! { #(#predicates,)* }
        }
        None => quote! {},
    };

    let deserialize_impl = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let deserialize_predicates = fields.named.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Deserialize<#ty, __D> })
                    }
                });
                let deserialize_predicates = quote! { #(#deserialize_predicates,)* };

                let deserialize_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote! { #name: self.#name.deserialize(deserializer)? }
                });

                quote! {
                    impl<__D: Fallible + ?Sized, #generic_params> Deserialize<#name<#generic_args>, __D> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #deserialize_predicates
                    {
                        fn deserialize(&self, deserializer: &mut __D) -> Result<#name<#generic_args>, __D::Error> {
                            Ok(#name::<#generic_args> {
                                #(#deserialize_fields,)*
                            })
                        }
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let deserialize_predicates = fields.unnamed.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Deserialize<#ty, __D> })
                    }
                });
                let deserialize_predicates = quote! { #(#deserialize_predicates,)* };

                let deserialize_fields = fields.unnamed.iter().enumerate().map(|(i, _)| {
                    let index = Index::from(i);
                    quote! { self.#index.deserialize(deserializer)? }
                });

                quote! {
                    impl<__D: Fallible + ?Sized, #generic_params> Deserialize<#name<#generic_args>, __D> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #deserialize_predicates
                    {
                        fn deserialize(&self, deserializer: &mut __D) -> Result<#name<#generic_args>, __D::Error> {
                            Ok(#name::<#generic_args>(
                                #(#deserialize_fields,)*
                            ))
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<__D: Fallible + ?Sized, #generic_params> Deserialize<#name<#generic_args>, __D> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                {
                    fn deserialize(&self, _: &mut __D) -> Result<#name<#generic_args>, __D::Error> {
                        Ok(#name::<#generic_args>)
                    }
                }
            },
        },
        Data::Enum(ref data) => {
            let deserialize_predicates = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let deserialize_predicates = fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Deserialize<#ty, __D> })
                        }
                    });
                    quote! { #(#deserialize_predicates,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let deserialize_predicates = fields.unnamed.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Deserialize<#ty, __D> })
                        }
                    });
                    quote! { #(#deserialize_predicates,)* }
                }
                Fields::Unit => quote! {}
            });
            let deserialize_predicates = quote! { #(#deserialize_predicates)* };

            let deserialize_variants = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            quote_spanned! { name.span() => #name }
                        });
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            quote! {
                                #name: #name.deserialize(deserializer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant { #(#bindings,)* } => #name::<#generic_args>::#variant { #(#fields,)* }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("_{}", i), f.span());
                            quote_spanned! { name.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let binding = Ident::new(&format!("_{}", i), f.span());
                            quote! {
                                #binding.deserialize(deserializer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant( #(#bindings,)* ) => #name::<#generic_args>::#variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => {
                        quote_spanned! { name.span() => Self::#variant => #name::<#generic_args>::#variant }
                    }
                }
            });

            quote! {
                impl<__D: Fallible + ?Sized, #generic_params> Deserialize<#name<#generic_args>, __D> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                    #deserialize_predicates
                {
                    fn deserialize(&self, deserializer: &mut __D) -> Result<#name<#generic_args>, __D::Error> {
                        Ok(match self {
                            #(#deserialize_variants,)*
                        })
                    }
                }
            }
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(input, "Deserialize cannot be derived for unions"))
        }
    };

    Ok(quote! {
        const _: () = {
            use rkyv::{Archive, Archived, Deserialize, Fallible};
            #deserialize_impl
        };
    })
}

fn derive_deserialize_copy_impl(input: &DeriveInput, attributes: &Attributes) -> Result<TokenStream, Error> {
    if let Some(ref archived) = attributes.archived {
        return Err(Error::new_spanned(archived, "archive copy types cannot be named"));
    } else if let Some(ref resolver) = attributes.resolver {
        return Err(Error::new_spanned(resolver, "archive copy resolvers cannot be named"));
    };

    let name = &input.ident;

    let generic_params = input
        .generics
        .params
        .iter()
        .map(|p| quote_spanned! { p.span() => #p });
    let generic_params = quote! { #(#generic_params,)* };

    let generic_args = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { name.span() => #name }
    });
    let generic_args = quote! { #(#generic_args,)* };

    let generic_predicates = match input.generics.where_clause {
        Some(ref clause) => {
            let predicates = clause.predicates.iter().map(|p| quote! { #p });
            quote! { #(#predicates,)* }
        }
        None => quote! {},
    };

    let deserialize_impl = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let deserialize_predicates = fields.named.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: ArchiveCopy })
                    }
                });
                let deserialize_predicates = quote! { #(#deserialize_predicates,)* };

                quote! {
                    impl<__D: Fallible + ?Sized, #generic_params> Deserialize<#name<#generic_args>, __D> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #deserialize_predicates
                    {
                        fn deserialize(&self, _: &mut __D) -> Result<Self, __D::Error> {
                            Ok(*self)
                        }
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let deserialize_predicates = fields.unnamed.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: ArchiveCopy })
                    }
                });
                let deserialize_predicates = quote! { #(#deserialize_predicates,)* };

                quote! {
                    impl<__D: Fallible + ?Sized, #generic_params> Deserialize<#name<#generic_args>, __D> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #deserialize_predicates
                    {
                        fn deserialize(&self, _: &mut __D) -> Result<Self, __D::Error> {
                            Ok(*self)
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<__D: Fallible + ?Sized, #generic_params> Deserialize<#name<#generic_args>, __D> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                {
                    fn deserialize(&self, _: &mut __D) -> Result<Self, __D::Error> {
                        Ok(*self)
                    }
                }
            },
        },
        Data::Enum(ref data) => {
            let deserialize_predicates = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let deserialize_predicates = fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: ArchiveCopy })
                        }
                    });
                    quote! { #(#deserialize_predicates,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let deserialize_predicates = fields.unnamed.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: ArchiveCopy })
                        }
                    });
                    quote! { #(#deserialize_predicates,)* }
                }
                Fields::Unit => quote! {},
            });
            let deserialize_predicates = quote! { #(#deserialize_predicates)* };

            quote! {
                impl<__D: Fallible + ?Sized, #generic_params> Deserialize<#name<#generic_args>, __D> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                    #deserialize_predicates
                {
                    fn deserialize(&self, _: &mut __D) -> Result<Self, __D::Error> {
                        Ok(*self)
                    }
                }
            }
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(input, "Deserialize cannot be derived for unions"));
        }
    };

    Ok(quote! {
        const _: () = {
            use rkyv::{Archive, Archived, ArchiveCopy, Deserialize, Fallible};
            #deserialize_impl
        };
    })
}
