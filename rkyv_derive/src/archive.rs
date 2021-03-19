use crate::attributes::{Attributes, parse_attributes};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Error, Fields, Ident, Index, spanned::Spanned};

pub fn derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = parse_attributes(&input)?;

    if attributes.copy.is_some() {
        derive_archive_copy_impl(&input, &attributes)
    } else {
        derive_archive_impl(&input, &attributes)
    }
}

fn derive_archive_impl(input: &DeriveInput, attributes: &Attributes) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let vis = &input.vis;

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

    let archive_derives = if let Some(derives) = attributes.derives.as_ref() {
        quote! { #[#derives] }
    } else {
        quote! {}
    };

    let archived = if let Some(ref archived) = attributes.archived {
        archived.clone()
    } else {
        Ident::new(&format!("Archived{}", name), name.span())
    };

    let resolver = if let Some(ref resolver) = attributes.resolver {
        resolver.clone()
    } else {
        Ident::new(&format!("{}Resolver", name), name.span())
    };

    let strict = if cfg!(feature = "strict") || attributes.strict.is_some() {
        quote! { #[repr(C)] }
    } else {
        quote! {}
    };

    let (archive_types, archive_impls) = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let archive_predicates = fields.named.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: rkyv::Archive })
                    }
                });
                let archive_predicates = quote! { #(#archive_predicates,)* };

                let resolver_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote_spanned! { f.span() => #name: rkyv::Resolver<#ty> }
                });

                let archived_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! { f.span() => #vis #name: rkyv::Archived<#ty> }
                });

                let archived_values = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! { f.span() => #name: self.#name.resolve(pos + offset_of!(#archived<#generic_args>, #name), resolver.#name) }
                });

                let mut compare_impls = Vec::new();
                if let Some((_, ref compares)) = attributes.compares {
                    for compare in compares {
                        if compare.is_ident("PartialEq") {
                            let partial_eq_predicates = fields.named.iter().map(|f| {
                                let ty = &f.ty;
                                quote_spanned! { f.span() => #ty: Archive, rkyv::Archived<#ty>: PartialEq<#ty> }
                            });
                            let partial_eq_predicates = quote! { #(#partial_eq_predicates,)* };

                            let field_names = fields.named.iter().map(|f| &f.ident);

                            compare_impls.push(quote! {
                                impl<#generic_params> PartialEq<#archived<#generic_args>> for #name<#generic_args>
                                where
                                    #generic_predicates
                                    #partial_eq_predicates
                                {
                                    #[inline]
                                    fn eq(&self, other: &#archived<#generic_args>) -> bool {
                                        #(other.#field_names == self.#field_names)&&*
                                    }
                                }

                                impl<#generic_params> PartialEq<#name<#generic_args>> for #archived<#generic_args>
                                where
                                    #generic_predicates
                                    #partial_eq_predicates
                                {
                                    #[inline]
                                    fn eq(&self, other: &#name<#generic_args>) -> bool {
                                        other.eq(self)
                                    }
                                }
                            });
                        } else {
                            return Err(Error::new_spanned(compare, "unrecognized compare argument, supported compares are PartialEq and PartialOrd"));
                        }
                    }
                }

                (
                    quote! {
                        #archive_derives
                        #strict
                        #vis struct #archived<#generic_params>
                        where
                            #generic_predicates
                            #archive_predicates
                        {
                            #(#archived_fields,)*
                        }

                        #vis struct #resolver<#generic_params>
                        where
                            #generic_predicates
                            #archive_predicates
                        {
                            #(#resolver_fields,)*
                        }
                    },
                    quote! {
                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #archive_predicates
                        {
                            type Archived = #archived<#generic_args>;
                            type Resolver = #resolver<#generic_args>;

                            fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
                                Self::Archived {
                                    #(#archived_values,)*
                                }
                            }
                        }

                        #(#compare_impls)*
                    },
                )
            }
            Fields::Unnamed(ref fields) => {
                let archive_predicates = fields.unnamed.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: rkyv::Archive })
                    }
                });
                let archive_predicates = quote! { #(#archive_predicates,)* };

                let resolver_fields = fields.unnamed.iter().map(|f| {
                    let ty = &f.ty;
                    quote_spanned! { f.span() => rkyv::Resolver<#ty> }
                });

                let archived_fields = fields.unnamed.iter().map(|f| {
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! { f.span() => #vis rkyv::Archived<#ty> }
                });

                let archived_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! { f.span() => self.#index.resolve(pos + offset_of!(#archived<#generic_args>, #index), resolver.#index) }
                });

                let mut compare_impls = Vec::new();
                if let Some((_, ref compares)) = attributes.compares {
                    for compare in compares {
                        if compare.is_ident("PartialEq") {
                            let partial_eq_predicates = fields.unnamed.iter().map(|f| {
                                let ty = &f.ty;
                                quote_spanned! { f.span() => #ty: Archive, rkyv::Archived<#ty>: PartialEq<#ty> }
                            });
                            let partial_eq_predicates = quote! { #(#partial_eq_predicates,)* };

                            let field_names = fields.unnamed.iter().enumerate().map(|(i, _)| Index::from(i));

                            compare_impls.push(quote! {
                                impl<#generic_params> PartialEq<#archived<#generic_args>> for #name<#generic_args>
                                where
                                    #generic_predicates
                                    #partial_eq_predicates
                                {
                                    #[inline]
                                    fn eq(&self, other: &#archived<#generic_args>) -> bool {
                                        #(other.#field_names == self.#field_names)&&*
                                    }
                                }

                                impl<#generic_params> PartialEq<#name<#generic_args>> for #archived<#generic_args>
                                where
                                    #generic_predicates
                                    #partial_eq_predicates
                                {
                                    #[inline]
                                    fn eq(&self, other: &#name<#generic_args>) -> bool {
                                        other.eq(self)
                                    }
                                }
                            });
                        } else {
                            return Err(Error::new_spanned(compare, "unrecognized compare argument, supported compares are PartialEq and PartialOrd"));
                        }
                    }
                }

                (
                    quote! {
                        #archive_derives
                        #strict
                        #vis struct #archived<#generic_params>(#(#archived_fields,)*)
                        where
                            #generic_predicates
                            #archive_predicates;

                        #vis struct #resolver<#generic_params>(#(#resolver_fields,)*)
                        where
                            #generic_predicates
                            #archive_predicates;
                    },
                    quote! {
                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #archive_predicates
                        {
                            type Archived = #archived<#generic_args>;
                            type Resolver = #resolver<#generic_args>;

                            fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
                                #archived::<#generic_args>(
                                    #(#archived_values,)*
                                )
                            }
                        }

                        #(#compare_impls)*
                    },
                )
            }
            Fields::Unit => {
                let mut compare_impls = Vec::new();
                if let Some((_, ref compares)) = attributes.compares {
                    for compare in compares {
                        if compare.is_ident("PartialEq") {
                            compare_impls.push(quote! {
                                impl<#generic_params> PartialEq<#archived<#generic_args>> for #name<#generic_args>
                                where
                                    #generic_predicates
                                {
                                    #[inline]
                                    fn eq(&self, _: &#archived<#generic_args>) -> bool {
                                        true
                                    }
                                }

                                impl<#generic_params> PartialEq<#name<#generic_args>> for #archived<#generic_args>
                                where
                                    #generic_predicates
                                {
                                    #[inline]
                                    fn eq(&self, _: &#name<#generic_args>) -> bool {
                                        true
                                    }
                                }
                            });
                        } else {
                            return Err(Error::new_spanned(compare, "unrecognized compare argument, supported compares are PartialEq and PartialOrd"));
                        }
                    }
                }

                (
                    quote! {
                        #archive_derives
                        #strict
                        #vis struct #archived<#generic_params>
                        where
                            #generic_predicates;

                        #vis struct #resolver<#generic_params>
                        where
                            #generic_predicates;
                    },
                    quote! {
                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                        {
                            type Archived = #archived<#generic_args>;
                            type Resolver = #resolver<#generic_args>;

                            fn resolve(&self, _pos: usize, _resolver: Self::Resolver) -> Self::Archived {
                                #archived::<#generic_args>
                            }
                        }

                        #(#compare_impls)*
                    },
                )
            },
        },
        Data::Enum(ref data) => {
            let archive_predicates = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let archive_predicates = fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: rkyv::Archive })
                        }
                    });
                    quote! { #(#archive_predicates,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let archive_predicates = fields.unnamed.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: rkyv::Archive })
                        }
                    });
                    quote! { #(#archive_predicates,)* }
                }
                Fields::Unit => quote! {},
            });
            let archive_predicates = quote! { #(#archive_predicates)* };

            let resolver_variants = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #name: rkyv::Resolver<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant {
                                #(#fields,)*
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => rkyv::Resolver<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => quote_spanned! { variant.span() => #variant },
                }
            });

            let resolve_arms = data.variants.iter().map(|v| {
                let variant = &v.ident;
                let archived_variant_name = Ident::new(&format!("ArchivedVariant{}", variant.to_string()), v.span());
                match v.fields {
                    Fields::Named(ref fields) => {
                        let self_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("self_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote_spanned! { name.span() => #name: #binding }
                        });
                        let resolver_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("resolver_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote_spanned! { binding.span() => #name: #binding }
                        });
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let self_binding = Ident::new(&format!("self_{}", name.as_ref().unwrap().to_string()), name.span());
                            let resolver_binding = Ident::new(&format!("resolver_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote! {
                                #name: #self_binding.resolve(pos + offset_of!(#archived_variant_name<#generic_args>, #name), #resolver_binding)
                            }
                        });
                        quote_spanned! { name.span() =>
                            #resolver::#variant { #(#resolver_bindings,)* } => {
                                if let #name::#variant { #(#self_bindings,)* } = self { #archived::#variant { #(#fields,)* } } else { panic!("enum resolver variant does not match value variant") }
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let self_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("self_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let resolver_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("resolver_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let index = Index::from(i + 1);
                            let self_binding = Ident::new(&format!("self_{}", i), f.span());
                            let resolver_binding = Ident::new(&format!("resolver_{}", i), f.span());
                            quote! {
                                #self_binding.resolve(pos + offset_of!(#archived_variant_name<#generic_args>, #index), #resolver_binding)
                            }
                        });
                        quote_spanned! { name.span() =>
                            #resolver::#variant( #(#resolver_bindings,)* ) => {
                                if let #name::#variant(#(#self_bindings,)*) = self { #archived::#variant(#(#fields,)*) } else { panic!("enum resolver variant does not match value variant") }
                            }
                        }
                    }
                    Fields::Unit => quote_spanned! { name.span() => #resolver::#variant => #archived::#variant }
                }
            });

            let archived_repr = match data.variants.len() {
                0..=255 => quote! { u8 },
                256..=65_535 => quote! { u16 },
                65_536..=4_294_967_295 => quote! { u32 },
                4_294_967_296..=18_446_744_073_709_551_615 => quote! { u64 },
                _ => quote! { u128 },
            };

            let archived_variants = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            let vis = &f.vis;
                            quote_spanned! { f.span() => #vis #name: rkyv::Archived<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant {
                                #(#fields,)*
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            let vis = &f.vis;
                            quote_spanned! { f.span() => #vis rkyv::Archived<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => quote_spanned! { variant.span() => #variant },
                }
            });

            let archived_variant_tags = data.variants.iter().map(|v| {
                let variant = &v.ident;
                quote_spanned! { variant.span() => #variant }
            });

            let archived_variant_structs = data.variants.iter().map(|v| {
                let variant = &v.ident;
                let archived_variant_name = Ident::new(&format!("ArchivedVariant{}", variant.to_string()), v.span());
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #name: rkyv::Archived<#ty> }
                        });
                        quote_spanned! { name.span() =>
                            #[repr(C)]
                            struct #archived_variant_name<#generic_params>
                            where
                                #generic_predicates
                                #archive_predicates
                            {
                                __tag: ArchivedTag,
                                #(#fields,)*
                                __phantom: PhantomData<(#generic_args)>,
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => rkyv::Archived<#ty> }
                        });
                        quote_spanned! { name.span() =>
                            #[repr(C)]
                            struct #archived_variant_name<#generic_params>(ArchivedTag, #(#fields,)* PhantomData<(#generic_args)>)
                            where
                                #generic_predicates
                                #archive_predicates;
                        }
                    }
                    Fields::Unit => quote! {}
                }
            });

            (
                quote! {
                    #archive_derives
                    #[repr(#archived_repr)]
                    #vis enum #archived<#generic_params>
                    where
                        #generic_predicates
                        #archive_predicates
                    {
                        #(#archived_variants,)*
                    }

                    #vis enum #resolver<#generic_params>
                    where
                        #generic_predicates
                        #archive_predicates
                    {
                        #(#resolver_variants,)*
                    }
                },
                quote! {
                    #[repr(#archived_repr)]
                    enum ArchivedTag {
                        #(#archived_variant_tags,)*
                    }

                    #(#archived_variant_structs)*

                    impl<#generic_params> Archive for #name<#generic_args>
                    where
                        #generic_predicates
                        #archive_predicates
                    {
                        type Archived = #archived<#generic_args>;
                        type Resolver = #resolver<#generic_args>;

                        fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
                            match resolver {
                                #(#resolve_arms,)*
                            }
                        }
                    }
                },
            )
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(input, "Archive cannot be derived for unions"))
        }
    };

    Ok(quote! {
        #archive_types

        const _: () = {
            use core::marker::PhantomData;
            use rkyv::{Archive, offset_of};

            #archive_impls
        };
    })
}

fn derive_archive_copy_impl(input: &DeriveInput, attributes: &Attributes) -> Result<TokenStream, Error> {
    if let Some(ref derives) = attributes.derives {
        return Err(Error::new_spanned(derives, "derives should be placed on the derived type for archive self derives"));
    }

    if let Some((ref compares, _)) = attributes.compares {
        return Err(Error::new_spanned(compares, "compares should be placed on the derived type for archive self derives"));
    }

    if let Some(ref archived) = attributes.archived {
        return Err(Error::new_spanned(archived, "archive copy types cannot be named"));
    } else if let Some(ref resolver) = attributes.resolver {
        return Err(Error::new_spanned(resolver, "archive copy resolvers cannot be named"));
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

    let archive_copy_impl = match input.data {
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
                unsafe impl<#generic_params> ArchiveCopy for #name<#generic_args>
                where
                    #generic_predicates
                    #copy_predicates
                {}

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #copy_predicates
                {
                    type Archived = Self;
                    type Resolver = ();

                    fn resolve(&self, _pos: usize, _resolver: Self::Resolver) -> Self::Archived {
                        *self
                    }
                }
            }
        }
        Data::Enum(ref data) => {
            if let Some(ref path) = attributes.repr.rust.as_ref()
                .or(attributes.repr.transparent.as_ref())
                .or(attributes.repr.packed.as_ref())
            {
                return Err(Error::new_spanned(path, "archive copy enums must be repr(C) or repr(Int)"))
            }

            if attributes.repr.c.is_none() && attributes.repr.int.is_none() {
                return Err(Error::new_spanned(input, "archive copy enums must be repr(C) or repr(Int)"));
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
                unsafe impl<#generic_params> ArchiveCopy for #name<#generic_args>
                where
                    #generic_predicates
                    #copy_predicates
                {}

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #copy_predicates
                {
                    type Archived = Self;
                    type Resolver = ();

                    fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
                        *self
                    }
                }
            }
        }
        Data::Union(_) => {
            Error::new(input.span(), "Archive cannot be derived for unions").to_compile_error()
        }
    };

    Ok(quote! {
        const _: () = {
            use rkyv::{
                Archive,
                ArchiveCopy,
                Serialize,
            };

            #archive_copy_impl
        };
    })
}