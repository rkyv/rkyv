//! Procedural macros for `rkyv`.

extern crate proc_macro;

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    parse_macro_input, spanned::Spanned, AttrStyle, Data, DeriveInput, Error, Fields, Ident, Index,
    Lit, Meta, MetaList, NestedMeta,
};

struct Repr {
    rust: Option<Span>,
    transparent: Option<Span>,
    packed: Option<Span>,
    c: Option<Span>,
    int: Option<Span>,
}

impl Default for Repr {
    fn default() -> Self {
        Self {
            rust: None,
            transparent: None,
            packed: None,
            c: None,
            int: None,
        }
    }
}

struct Attributes {
    copy: Option<Span>,
    repr: Repr,
    derives: Option<MetaList>,
    name: Option<(Ident, Span)>,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            copy: None,
            repr: Default::default(),
            derives: None,
            name: None,
        }
    }
}

fn parse_attributes(input: &DeriveInput) -> Result<Attributes, TokenStream> {
    let mut result = Attributes::default();
    for a in input.attrs.iter() {
        if let AttrStyle::Outer = a.style {
            if let Ok(Meta::List(meta)) = a.parse_meta() {
                if meta.path.is_ident("archive") {
                    for n in meta.nested.iter() {
                        if let NestedMeta::Meta(meta) = n {
                            match meta {
                                Meta::Path(path) => {
                                    if path.is_ident("copy") {
                                        if result.copy.is_none() {
                                            result.copy = Some(path.span());
                                        } else {
                                            return Err(Error::new(
                                                meta.span(),
                                                "copy already specified",
                                            )
                                            .to_compile_error());
                                        }
                                    } else {
                                        return Err(Error::new(
                                            path.span(),
                                            "unrecognized archive parameter",
                                        )
                                        .to_compile_error());
                                    }
                                }
                                Meta::List(meta) => {
                                    if meta.path.is_ident("derive") {
                                        result.derives = Some(meta.clone());
                                    } else {
                                        return Err(Error::new(
                                            meta.path.span(),
                                            "unrecognized archive parameter",
                                        )
                                        .to_compile_error());
                                    }
                                }
                                Meta::NameValue(meta) => {
                                    if meta.path.is_ident("name") {
                                        if let Lit::Str(ref lit_str) = meta.lit {
                                            if result.name.is_none() {
                                                result.name = Some((
                                                    Ident::new(
                                                        &lit_str.value(),
                                                        lit_str.span(),
                                                    ),
                                                    lit_str.span(),
                                                ));
                                            } else {
                                                return Err(Error::new(
                                                    meta.span(),
                                                    "name already specified",
                                                )
                                                .to_compile_error());
                                            }
                                        } else {
                                            return Err(Error::new(
                                                meta.span(),
                                                "name must be a string",
                                            )
                                            .to_compile_error());
                                        }
                                    } else {
                                        return Err(Error::new(
                                            meta.span(),
                                            "unrecognized archive parameter",
                                        )
                                        .to_compile_error());
                                    }
                                }
                            }
                        }
                    }
                } else if meta.path.is_ident("repr") {
                    for n in meta.nested.iter() {
                        if let NestedMeta::Meta(Meta::Path(path)) = n {
                            if path.is_ident("rust") {
                                result.repr.rust = Some(path.span());
                            } else if path.is_ident("transparent") {
                                result.repr.transparent = Some(path.span());
                            } else if path.is_ident("packed") {
                                result.repr.packed = Some(path.span());
                            } else if path.is_ident("C") {
                                result.repr.c = Some(path.span());
                            } else {
                                result.repr.int = Some(path.span());
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(result)
}

/// Derives `Archive` for the labeled type.
///
/// Additional arguments can be specified using the `#[archive(...)]` attribute:
///
/// - `copy`: Implements `ArchiveCopy` as well as `Archive`. Only suitable for
/// types that can be directly archived.
/// - `derive(...)`: Adds a `#[derive(...)]` attribute to the archived type.
/// - `name`, `name = "..."`: Exposes the archived type with the given name. If
/// used without a name assignment, uses the name `"Archived" + name`.
///
/// This derive macro automatically adds a type bound `field: Archive` for each
/// field type. This can cause an overflow while evaluating trait bounds if the
/// structure eventually references its own type, as the implementation of
/// `Archive` for a struct depends on each field type implementing it as well.
/// Adding the attribute `#[recursive]` to a field will suppress this trait
/// bound and allow recursive structures. This may be too coarse for some types,
/// in which case `Archive` will have to be implemented manually.
///
/// # Example
///
/// ```
/// use rkyv::Archive;
///
/// #[derive(Archive)]
/// enum Node<T> {
///     Nil,
///     Cons(T, #[recursive] Box<Node<T>>),
/// }
/// ```
#[proc_macro_derive(Archive, attributes(archive, recursive))]
pub fn archive_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let attributes = match parse_attributes(&input) {
        Ok(attributes) => attributes,
        Err(errors) => return proc_macro::TokenStream::from(errors),
    };

    let archive_impl = if attributes.copy.is_some() {
        derive_archive_copy_impl(&input, &attributes)
    } else {
        derive_archive_impl(&input, &attributes)
    };

    proc_macro::TokenStream::from(archive_impl)
}

fn derive_archive_impl(input: &DeriveInput, attributes: &Attributes) -> TokenStream {
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

    let archived = if let Some((ref name, _)) = attributes.name {
        name.clone()
    } else {
        Ident::new(&format!("Archived{}", name), name.span())
    };

    let resolver = Ident::new(&format!("{}Resolver", name), name.span());

    #[cfg(feature = "strict")]
    let strict = quote! { #[repr(C)] };
    #[cfg(not(feature = "strict"))]
    let strict = quote! {};

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
                    },
                )
            }
            Fields::Unit => (
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
                },
            ),
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
            return Error::new(input.span(), "Archive cannot be derived for unions")
                .to_compile_error()
        }
    };

    quote! {
        #archive_types

        const _: () = {
            use core::marker::PhantomData;
            use rkyv::{
                Archive,
                offset_of,
                Serialize,
                Write,
            };
            #archive_impls
        };
    }
}

fn derive_archive_copy_impl(input: &DeriveInput, attributes: &Attributes) -> TokenStream {
    if let Some(derives) = &attributes.derives {
        return Error::new(
            derives.span(),
            "derives should be placed on the derived type for archive self derives",
        )
        .to_compile_error();
    }

    if let Some((_, span)) = &attributes.name {
        return Error::new(*span, "archive self types cannot be named").to_compile_error();
    }

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
            if let Some(span) = attributes
                .repr
                .rust
                .or(attributes.repr.transparent)
                .or(attributes.repr.packed)
            {
                return Error::new(span, "archive copy enums must be repr(C) or repr(Int)")
                    .to_compile_error();
            }

            if attributes.repr.c.is_none() && attributes.repr.int.is_none() {
                return Error::new(
                    input.span(),
                    "archive copy enums must be repr(C) or repr(Int)",
                )
                .to_compile_error();
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

                    fn resolve(&self, _pos: usize, _resolver: Self::Resolver) -> Self::Archived {
                        *self
                    }
                }
            }
        }
        Data::Union(_) => {
            Error::new(input.span(), "Archive cannot be derived for unions").to_compile_error()
        }
    };

    quote! {
        const _: () = {
            use rkyv::{
                Archive,
                ArchiveCopy,
                Serialize,
                Write,
            };

            #archive_copy_impl
        };
    }
}

#[proc_macro_derive(Serialize, attributes(archive, recursive))]
pub fn serialize_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let attributes = match parse_attributes(&input) {
        Ok(attributes) => attributes,
        Err(errors) => return proc_macro::TokenStream::from(errors),
    };

    let serialize_impl = if attributes.copy.is_some() {
        derive_serialize_copy_impl(&input, &attributes)
    } else {
        derive_serialize_impl(&input)
    };

    proc_macro::TokenStream::from(serialize_impl)
}

fn derive_serialize_impl(input: &DeriveInput) -> TokenStream {
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

    let resolver = Ident::new(&format!("{}Resolver", name), name.span());

    let serialize_impl = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let serialize_predicates = fields.named.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: rkyv::Serialize<__W> })
                    }
                });
                let serialize_predicates = quote! { #(#serialize_predicates,)* };

                let resolver_values = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! { f.span() => #name: Serialize::<__W>::serialize(&self.#name, writer)? }
                });

                quote! {
                    impl<__W: Write + ?Sized, #generic_params> Serialize<__W> for #name<#generic_args>
                    where
                        #generic_predicates
                        #serialize_predicates
                    {
                        fn serialize(&self, writer: &mut __W) -> Result<Self::Resolver, __W::Error> {
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
                        Some(quote_spanned! { f.span() => #ty: rkyv::Serialize<__W> })
                    }
                });
                let serialize_predicates = quote! { #(#serialize_predicates,)* };

                let resolver_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! { f.span() => Serialize::<__W>::serialize(&self.#index, writer)? }
                });

                quote! {
                    impl<__W: Write + ?Sized, #generic_params> Serialize<__W> for #name<#generic_args>
                    where
                        #generic_predicates
                        #serialize_predicates
                    {
                        fn serialize(&self, writer: &mut __W) -> Result<Self::Resolver, __W::Error> {
                            Ok(#resolver::<#generic_args>(
                                #(#resolver_values,)*
                            ))
                        }
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    impl<__W: Write + ?Sized, #generic_params> Serialize<__W> for #name<#generic_args> {
                        fn serialize(&self, writer: &mut __W) -> Result<Self::Resolver, __W::Error> {
                            Ok(#resolver)
                        }
                    }
                }
            }
        }
        Data::Enum(ref data) => {
            let serialize_predicates = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let serialize_predicates = fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: rkyv::Serialize<__W> })
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
                            Some(quote_spanned! { f.span() => #ty: rkyv::Serialize<__W> })
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
                                #name: Serialize::<__W>::serialize(#name, writer)?
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
                                Serialize::<__W>::serialize(#binding, writer)?
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
                impl<__W: Write + ?Sized, #generic_params> Serialize<__W> for #name<#generic_args>
                where
                    #generic_predicates
                    #serialize_predicates
                {
                    fn serialize(&self, writer: &mut __W) -> Result<Self::Resolver, __W::Error> {
                        Ok(match self {
                            #(#serialize_arms,)*
                        })
                    }
                }
            }
        }
        Data::Union(_) => {
            return Error::new(input.span(), "Serialize cannot be derived for unions").to_compile_error()
        }
    };

    quote! {
        const _: () = {
            use rkyv::{
                Archive,
                Serialize,
                Write
            };
            #serialize_impl
        };
    }
}

fn derive_serialize_copy_impl(input: &DeriveInput, attributes: &Attributes) -> TokenStream {
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
                impl<__W: Write + ?Sized, #generic_params> Serialize<__W> for #name<#generic_args>
                where
                    #generic_predicates
                    #copy_predicates
                {
                    fn serialize(&self, writer: &mut __W) -> Result<Self::Resolver, __W::Error> {
                        Ok(())
                    }
                }
            }
        }
        Data::Enum(ref data) => {
            if let Some(span) = attributes
                .repr
                .rust
                .or(attributes.repr.transparent)
                .or(attributes.repr.packed)
            {
                return Error::new(span, "archive copy enums must be repr(C) or repr(Int)")
                    .to_compile_error();
            }

            if attributes.repr.c.is_none() && attributes.repr.int.is_none() {
                return Error::new(
                    input.span(),
                    "archive copy enums must be repr(C) or repr(Int)",
                )
                .to_compile_error();
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
                impl<__W: Write + ?Sized, #generic_params> Serialize<__W> for #name<#generic_args>
                where
                    #generic_predicates
                    #copy_predicates
                {
                    fn serialize(&self, writer: &mut __W) -> Result<Self::Resolver, __W::Error> {
                        Ok(())
                    }
                }
            }
        }
        Data::Union(_) => {
            Error::new(input.span(), "Serialize cannot be derived for unions").to_compile_error()
        }
    };

    quote! {
        const _: () = {
            use rkyv::{
                Archive,
                ArchiveCopy,
                Serialize,
                Write,
            };

            #serialize_copy_impl
        };
    }
}

/// Derives `Deserialize` for the labeled type.
///
/// This macro also supports the `#[recursive]` attribute. See [`Archive`] for
/// more information.
#[proc_macro_derive(Deserialize, attributes(recursive))]
pub fn deserialize_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let attributes = match parse_attributes(&input) {
        Ok(attributes) => attributes,
        Err(errors) => return proc_macro::TokenStream::from(errors),
    };

    let deserialize_impl = if attributes.copy.is_some() {
        derive_deserialize_copy_impl(&input)
    } else {
        derive_deserialize_impl(&input)
    };

    proc_macro::TokenStream::from(deserialize_impl)
}

fn derive_deserialize_impl(input: &DeriveInput) -> TokenStream {
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
                        Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Deserialize<#ty, __C> })
                    }
                });
                let deserialize_predicates = quote! { #(#deserialize_predicates,)* };

                let deserialize_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote! { #name: self.#name.deserialize(context) }
                });

                quote! {
                    impl<__C: ?Sized, #generic_params> Deserialize<#name<#generic_args>, __C> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #deserialize_predicates
                    {
                        fn deserialize(&self, context: &mut __C) -> #name<#generic_args> {
                            #name::<#generic_args> {
                                #(#deserialize_fields,)*
                            }
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
                        Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Deserialize<#ty, __C> })
                    }
                });
                let deserialize_predicates = quote! { #(#deserialize_predicates,)* };

                let deserialize_fields = fields.unnamed.iter().enumerate().map(|(i, _)| {
                    let index = Index::from(i);
                    quote! { self.#index.deserialize(context) }
                });

                quote! {
                    impl<__C: ?Sized, #generic_params> Deserialize<#name<#generic_args>, __C> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #deserialize_predicates
                    {
                        fn deserialize(&self, context: &mut __C) -> #name<#generic_args> {
                            #name::<#generic_args>(
                                #(#deserialize_fields,)*
                            )
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<__C: ?Sized, #generic_params> Deserialize<#name<#generic_args>, __C> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                {
                    fn deserialize(&self, _: &mut __C) -> #name<#generic_args> {
                        #name::<#generic_args>
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
                            Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Deserialize<#ty, __C> })
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
                            Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Deserialize<#ty, __C> })
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
                                #name: #name.deserialize(context)
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
                                #binding.deserialize(context)
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
                impl<__C: ?Sized, #generic_params> Deserialize<#name<#generic_args>, __C> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                    #deserialize_predicates
                {
                    fn deserialize(&self, context: &mut __C) -> #name<#generic_args> {
                        match self {
                            #(#deserialize_variants,)*
                        }
                    }
                }
            }
        }
        Data::Union(_) => {
            return Error::new(input.span(), "Deserialize cannot be derived for unions")
                .to_compile_error()
        }
    };

    quote! {
        const _: () = {
            use rkyv::{Archive, Archived, Deserialize};
            #deserialize_impl
        };
    }
}

fn derive_deserialize_copy_impl(input: &DeriveInput) -> TokenStream {
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
                    impl<__C: ?Sized, #generic_params> Deserialize<#name<#generic_args>, __C> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #deserialize_predicates
                    {
                        fn deserialize(&self, _: &mut __C) -> Self {
                            *self
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
                    impl<__C: ?Sized, #generic_params> Deserialize<#name<#generic_args>, __C> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #deserialize_predicates
                    {
                        fn deserialize(&self, _: &mut __C) -> Self {
                            *self
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<__C: ?Sized, #generic_params> Deserialize<#name<#generic_args>, __C> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                {
                    fn deserialize(&self, _: &mut __C) -> Self {
                        *self
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
                impl<__C: ?Sized, #generic_params> Deserialize<#name<#generic_args>, __C> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                    #deserialize_predicates
                {
                    fn deserialize(&self, _: &mut __C) -> Self {
                        *self
                    }
                }
            }
        }
        Data::Union(_) => {
            return Error::new(input.span(), "Deserialize cannot be derived for unions")
                .to_compile_error()
        }
    };

    quote! {
        const _: () = {
            use rkyv::{Archive, Archived, ArchiveCopy, Deserialize};
            #deserialize_impl
        };
    }
}
