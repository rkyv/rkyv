use crate::attributes::{parse_attributes, Attributes};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DeriveInput, Error, Fields, Ident, Index};

pub fn derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = parse_attributes(&input)?;

    if attributes.copy.is_some() {
        derive_serialize_copy_impl(&input, &attributes)
    } else {
        derive_serialize_impl(&input, &attributes)
    }
}

fn derive_serialize_impl(
    input: &DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
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

    let resolver = if let Some(ref resolver) = attributes.resolver {
        resolver.clone()
    } else {
        Ident::new(&format!("{}Resolver", name), name.span())
    };

    let serialize_impl = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let serialize_predicates = fields.named.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: rkyv::Serialize<__S> })
                    }
                });
                let serialize_predicates = quote! { #(#serialize_predicates,)* };

                let resolver_values = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! { f.span() => #name: Serialize::<__S>::serialize(&self.#name, serializer)? }
                });

                quote! {
                    impl<__S: Fallible + ?Sized, #generic_params> Serialize<__S> for #name<#generic_args>
                    where
                        #generic_predicates
                        #serialize_predicates
                    {
                        fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                            Ok(#resolver {
                                #(#resolver_values,)*
                            })
                        }
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let serialize_predicates = fields.unnamed.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: rkyv::Serialize<__S> })
                    }
                });
                let serialize_predicates = quote! { #(#serialize_predicates,)* };

                let resolver_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! { f.span() => Serialize::<__S>::serialize(&self.#index, serializer)? }
                });

                quote! {
                    impl<__S: Fallible + ?Sized, #generic_params> Serialize<__S> for #name<#generic_args>
                    where
                        #generic_predicates
                        #serialize_predicates
                    {
                        fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                            Ok(#resolver::<#generic_args>(
                                #(#resolver_values,)*
                            ))
                        }
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    impl<__S: Fallible + ?Sized, #generic_params> Serialize<__S> for #name<#generic_args> {
                        fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                            Ok(#resolver)
                        }
                    }
                }
            }
        },
        Data::Enum(ref data) => {
            let serialize_predicates = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let serialize_predicates = fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: rkyv::Serialize<__S> })
                        }
                    });
                    quote! { #(#serialize_predicates,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let serialize_predicates = fields.unnamed.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: rkyv::Serialize<__S> })
                        }
                    });
                    quote! { #(#serialize_predicates,)* }
                }
                Fields::Unit => quote! {},
            });
            let serialize_predicates = quote! { #(#serialize_predicates)* };

            let serialize_arms = data.variants.iter().map(|v| {
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
                                #name: Serialize::<__S>::serialize(#name, serializer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant { #(#bindings,)* } => #resolver::#variant {
                                #(#fields,)*
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let binding = Ident::new(&format!("_{}", i), f.span());
                            quote! {
                                Serialize::<__S>::serialize(#binding, serializer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant( #(#bindings,)* ) => #resolver::#variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => {
                        quote_spanned! { name.span() => Self::#variant => #resolver::#variant }
                    }
                }
            });

            quote! {
                impl<__S: Fallible + ?Sized, #generic_params> Serialize<__S> for #name<#generic_args>
                where
                    #generic_predicates
                    #serialize_predicates
                {
                    fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                        Ok(match self {
                            #(#serialize_arms,)*
                        })
                    }
                }
            }
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Serialize cannot be derived for unions",
            ))
        }
    };

    Ok(quote! {
        const _: () = {
            use rkyv::{
                Archive,
                Serialize,
                Fallible,
            };
            #serialize_impl
        };
    })
}

fn derive_serialize_copy_impl(
    input: &DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    if let Some(ref archived) = attributes.archived {
        return Err(Error::new_spanned(
            archived,
            "archive copy types cannot be named",
        ));
    } else if let Some(ref resolver) = attributes.resolver {
        return Err(Error::new_spanned(
            resolver,
            "archive copy resolvers cannot be named",
        ));
    };

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
        }
        None => quote! {},
    };

    let serialize_copy_impl = match input.data {
        Data::Struct(ref data) => {
            let copy_predicates = match data.fields {
                Fields::Named(ref fields) => {
                    let copy_predicates = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });

                    quote! { #(#copy_predicates,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let copy_predicates = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });

                    quote! { #(#copy_predicates,)* }
                }
                Fields::Unit => quote! {},
            };

            quote! {
                impl<__S: Fallible + ?Sized, #generic_params> Serialize<__S> for #name<#generic_args>
                where
                    #generic_predicates
                    #copy_predicates
                {
                    fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                        Ok(())
                    }
                }
            }
        }
        Data::Enum(ref data) => {
            if let Some(ref path) = attributes
                .repr
                .rust
                .as_ref()
                .or_else(|| attributes.repr.transparent.as_ref())
                .or_else(|| attributes.repr.packed.as_ref())
            {
                return Err(Error::new_spanned(
                    path,
                    "archive copy enums must be repr(C) or repr(Int)",
                ));
            }

            if attributes.repr.c.is_none() && attributes.repr.int.is_none() {
                return Err(Error::new_spanned(
                    input,
                    "archive copy enums must be repr(C) or repr(Int)",
                ));
            }

            let copy_predicates = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let copy_predicates = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });
                    quote! { #(#copy_predicates,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let copy_predicates = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });
                    quote! { #(#copy_predicates,)* }
                }
                Fields::Unit => quote! {},
            });
            let copy_predicates = quote! { #(#copy_predicates)* };

            quote! {
                impl<__S: Fallible + ?Sized, #generic_params> Serialize<__S> for #name<#generic_args>
                where
                    #generic_predicates
                    #copy_predicates
                {
                    fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                        Ok(())
                    }
                }
            }
        }
        Data::Union(_) => {
            Error::new(input.span(), "Serialize cannot be derived for unions").to_compile_error()
        }
    };

    Ok(quote! {
        const _: () = {
            use rkyv::{
                Archive,
                ArchiveCopy,
                Serialize,
                Fallible,
            };

            #serialize_copy_impl
        };
    })
}
