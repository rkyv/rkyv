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
    archive_copy: Option<Span>,
    repr: Repr,
    derives: Option<MetaList>,
    name: Option<(Option<Ident>, Span)>,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            archive_copy: None,
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
                                        if result.archive_copy.is_none() {
                                            result.archive_copy = Some(path.span());
                                        } else {
                                            return Err(Error::new(
                                                meta.span(),
                                                "copy already specified",
                                            )
                                            .to_compile_error());
                                        }
                                    } else if path.is_ident("name") {
                                        if result.name.is_none() {
                                            result.name = Some((None, path.span()));
                                        } else {
                                            return Err(Error::new(
                                                meta.span(),
                                                "name already specified",
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
                                                    Some(Ident::new(
                                                        &lit_str.value(),
                                                        lit_str.span(),
                                                    )),
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

    let archive_impl = if attributes.archive_copy.is_some() {
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

    let archived = if let Some((Some(ref name), _)) = attributes.name {
        name.clone()
    } else {
        Ident::new(&format!("Archived{}", name), name.span())
    };

    #[cfg(feature = "strict")]
    let strict = quote! { #[repr(C)] };
    #[cfg(not(feature = "strict"))]
    let strict = quote! {};

    let (archive_type, archive_impl) = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let field_wheres = fields.named.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: rkyv::Archive })
                    }
                });
                let field_wheres = quote! { #(#field_wheres,)* };

                let resolver_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote_spanned! { f.span() => #name: rkyv::Resolver<#ty> }
                });

                let resolver_values = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! { f.span() => #name: self.#name.archive(writer)? }
                });

                let archived_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! { f.span() => #vis #name: rkyv::Archived<#ty> }
                });

                let archived_values = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! { f.span() => #name: self.#name.resolve(pos + offset_of!(#archived<#generic_args>, #name), &value.#name) }
                });

                (
                    quote! {
                        #archive_derives
                        #strict
                        #vis struct #archived<#generic_params>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            #(#archived_fields,)*
                        }
                    },
                    quote! {
                        #vis struct Resolver<#generic_params>
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
                            type Archived = #archived<#generic_args>;

                            fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                                Self::Archived {
                                    #(#archived_values,)*
                                }
                            }
                        }

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = #archived<#generic_args>;
                            type Resolver = Resolver<#generic_args>;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Self::Resolver {
                                    #(#resolver_values,)*
                                })
                            }
                        }
                    },
                )
            }
            Fields::Unnamed(ref fields) => {
                let field_wheres = fields.unnamed.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: rkyv::Archive })
                    }
                });
                let field_wheres = quote! { #(#field_wheres,)* };

                let resolver_fields = fields.unnamed.iter().map(|f| {
                    let ty = &f.ty;
                    quote_spanned! { f.span() => rkyv::Resolver<#ty> }
                });

                let resolver_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! { f.span() => self.#index.archive(writer)? }
                });

                let archived_fields = fields.unnamed.iter().map(|f| {
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! { f.span() => #vis rkyv::Archived<#ty> }
                });

                let archived_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! { f.span() => self.#index.resolve(pos + offset_of!(#archived<#generic_args>, #index), &value.#index) }
                });

                (
                    quote! {
                        #archive_derives
                        #strict
                        #vis struct #archived<#generic_params>(#(#archived_fields,)*)
                        where
                            #generic_predicates
                            #field_wheres;
                    },
                    quote! {
                        #vis struct Resolver<#generic_params>(#(#resolver_fields,)*)
                        where
                            #generic_predicates
                            #field_wheres;

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = #archived<#generic_args>;

                            fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                                #archived::<#generic_args>(
                                    #(#archived_values,)*
                                )
                            }
                        }

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = #archived<#generic_args>;
                            type Resolver = Resolver<#generic_args>;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Resolver::<#generic_args>(
                                    #(#resolver_values,)*
                                ))
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
                },
                quote! {
                    #vis struct Resolver;

                    impl<#generic_params> Resolve<#name<#generic_args>> for Resolver
                    where
                        #generic_predicates
                    {
                        type Archived = #archived<#generic_args>;

                        fn resolve(self, _pos: usize, _value: &#name<#generic_args>) -> Self::Archived {
                            #archived::<#generic_args>
                        }
                    }

                    impl<#generic_params> Archive for #name<#generic_args>
                    where
                        #generic_predicates
                    {
                        type Archived = #archived<#generic_args>;
                        type Resolver = Resolver;

                        fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                            Ok(Resolver)
                        }
                    }
                },
            ),
        },
        Data::Enum(ref data) => {
            let field_wheres = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: rkyv::Archive })
                        }
                    });
                    quote! { #(#field_wheres,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: rkyv::Archive })
                        }
                    });
                    quote! { #(#field_wheres,)* }
                }
                Fields::Unit => quote! {},
            });
            let field_wheres = quote! { #(#field_wheres)* };

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
                        let value_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("value_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote_spanned! { binding.span() => #name: #binding }
                        });
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let self_binding = Ident::new(&format!("self_{}", name.as_ref().unwrap().to_string()), name.span());
                            let value_name = Ident::new(&format!("value_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote! {
                                #name: #self_binding.resolve(pos + offset_of!(#archived_variant_name<#generic_args>, #name), #value_name)
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant { #(#self_bindings,)* } => {
                                if let #name::#variant { #(#value_bindings,)* } = value { #archived::#variant { #(#fields,)* } } else { panic!("enum resolver variant does not match value variant") }
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let self_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("self_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let value_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("value_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let index = Index::from(i + 1);
                            let self_binding = Ident::new(&format!("self_{}", i), f.span());
                            let value_binding = Ident::new(&format!("value_{}", i), f.span());
                            quote! {
                                #self_binding.resolve(pos + offset_of!(#archived_variant_name<#generic_args>, #index), #value_binding)
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant( #(#self_bindings,)* ) => {
                                if let #name::#variant(#(#value_bindings,)*) = value { #archived::#variant(#(#fields,)*) } else { panic!("enum resolver variant does not match value variant") }
                            }
                        }
                    }
                    Fields::Unit => quote_spanned! { name.span() => Self::#variant => #archived::#variant }
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
                                #field_wheres
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
                                #field_wheres;
                        }
                    }
                    Fields::Unit => quote! {}
                }
            });

            let archive_arms = data.variants.iter().map(|v| {
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
                                #name: #name.archive(writer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant { #(#bindings,)* } => Resolver::#variant {
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
                                #binding.archive(writer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant( #(#bindings,)* ) => Resolver::#variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => {
                        quote_spanned! { name.span() => Self::#variant => Resolver::#variant }
                    }
                }
            });

            (
                quote! {
                    #archive_derives
                    #[repr(#archived_repr)]
                    #vis enum #archived<#generic_params>
                    where
                        #generic_predicates
                        #field_wheres
                    {
                        #(#archived_variants,)*
                    }
                },
                quote! {
                    #vis enum Resolver<#generic_params>
                    where
                        #generic_predicates
                        #field_wheres
                    {
                        #(#resolver_variants,)*
                    }

                    impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                    where
                        #generic_predicates
                        #field_wheres
                    {
                        type Archived = #archived<#generic_args>;

                        fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                            match self {
                                #(#resolve_arms,)*
                            }
                        }
                    }

                    #[repr(#archived_repr)]
                    enum ArchivedTag {
                        #(#archived_variant_tags,)*
                    }

                    #(#archived_variant_structs)*

                    impl<#generic_params> Archive for #name<#generic_args>
                    where
                        #generic_predicates
                        #field_wheres
                    {
                        type Archived = #archived<#generic_args>;
                        type Resolver = Resolver<#generic_args>;

                        fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                            Ok(match self {
                                #(#archive_arms,)*
                            })
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

    if attributes.name.is_some() {
        quote! {
            #archive_type

            const _: () = {
                use core::marker::PhantomData;
                use rkyv::{
                    Archive,
                    offset_of,
                    Resolve,
                    Write,
                };
                #archive_impl
            };
        }
    } else {
        quote! {
            const _: () = {
                use core::marker::PhantomData;
                use rkyv::{
                    Archive,
                    offset_of,
                    Resolve,
                    Write,
                };
                #archive_type
                #archive_impl
            };
        }
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

    let archive_self_impl = match input.data {
        Data::Struct(ref data) => {
            let field_wheres = match data.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });

                    quote! { #(#field_wheres,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });

                    quote! { #(#field_wheres,)* }
                }
                Fields::Unit => quote! {},
            };

            quote! {
                unsafe impl<#generic_params> ArchiveCopy for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {}

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Self;
                    type Resolver = CopyResolver;

                    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                        Ok(CopyResolver)
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
                return Error::new(span, "archive self enums must be repr(C) or repr(Int)")
                    .to_compile_error();
            }

            if attributes.repr.c.is_none() && attributes.repr.int.is_none() {
                return Error::new(
                    input.span(),
                    "archive self enums must be repr(C) or repr(Int)",
                )
                .to_compile_error();
            }

            let field_wheres = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });
                    quote! { #(#field_wheres,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveCopy }
                    });
                    quote! { #(#field_wheres,)* }
                }
                Fields::Unit => quote! {},
            });
            let field_wheres = quote! { #(#field_wheres)* };

            quote! {
                unsafe impl<#generic_params> ArchiveCopy for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {}

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Self;
                    type Resolver = CopyResolver;

                    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                        Ok(CopyResolver)
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
                CopyResolver,
                Write,
            };

            #archive_self_impl
        };
    }
}

/// Derives `Unarchive` for the labeled type.
///
/// This macro also supports the `#[recursive]` attribute. See [`Archive`] for
/// more information.
#[proc_macro_derive(Unarchive, attributes(recursive))]
pub fn unarchive_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let attributes = match parse_attributes(&input) {
        Ok(attributes) => attributes,
        Err(errors) => return proc_macro::TokenStream::from(errors),
    };

    let unarchive_impl = if attributes.archive_copy.is_some() {
        derive_unarchive_copy_impl(&input)
    } else {
        derive_unarchive_impl(&input)
    };

    proc_macro::TokenStream::from(unarchive_impl)
}

fn derive_unarchive_impl(input: &DeriveInput) -> TokenStream {
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

    let unarchive_impl = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let field_wheres = fields.named.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Unarchive<#ty> })
                    }
                });
                let field_wheres = quote! { #(#field_wheres,)* };

                let unarchive_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote! { #name: self.#name.unarchive() }
                });

                quote! {
                    impl<#generic_params> Unarchive<#name<#generic_args>> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #field_wheres
                    {
                        fn unarchive(&self) -> #name<#generic_args> {
                            #name::<#generic_args> {
                                #(#unarchive_fields,)*
                            }
                        }
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let field_wheres = fields.unnamed.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Unarchive<#ty> })
                    }
                });
                let field_wheres = quote! { #(#field_wheres,)* };

                let unarchive_fields = fields.unnamed.iter().enumerate().map(|(i, _)| {
                    let index = Index::from(i);
                    quote! { self.#index.unarchive() }
                });

                quote! {
                    impl<#generic_params> Unarchive<#name<#generic_args>> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #field_wheres
                    {
                        fn unarchive(&self) -> #name<#generic_args> {
                            #name::<#generic_args>(
                                #(#unarchive_fields,)*
                            )
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<#generic_params> Unarchive<#name<#generic_args>> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                {
                    fn unarchive(&self) -> #name<#generic_args> {
                        #name::<#generic_args>
                    }
                }
            },
        },
        Data::Enum(ref data) => {
            let field_wheres = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Unarchive<#ty> })
                        }
                    });
                    quote! { #(#field_wheres,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: Archive, Archived<#ty>: Unarchive<#ty> })
                        }
                    });
                    quote! { #(#field_wheres,)* }
                }
                Fields::Unit => quote! {}
            });
            let field_wheres = quote! { #(#field_wheres)* };

            let unarchive_variants = data.variants.iter().map(|v| {
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
                                #name: #name.unarchive()
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
                                #binding.unarchive()
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
                impl<#generic_params> Unarchive<#name<#generic_args>> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                    #field_wheres
                {
                    fn unarchive(&self) -> #name<#generic_args> {
                        match self {
                            #(#unarchive_variants,)*
                        }
                    }
                }
            }
        }
        Data::Union(_) => {
            return Error::new(input.span(), "Unarchive cannot be derived for unions")
                .to_compile_error()
        }
    };

    quote! {
        const _: () = {
            use rkyv::{Archive, Archived, Unarchive};
            #unarchive_impl
        };
    }
}

fn derive_unarchive_copy_impl(input: &DeriveInput) -> TokenStream {
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

    let unarchive_impl = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let field_wheres = fields.named.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: ArchiveCopy })
                    }
                });
                let field_wheres = quote! { #(#field_wheres,)* };

                quote! {
                    impl<#generic_params> Unarchive<#name<#generic_args>> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #field_wheres
                    {
                        fn unarchive(&self) -> Self {
                            *self
                        }
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let field_wheres = fields.unnamed.iter().filter_map(|f| {
                    if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                        None
                    } else {
                        let ty = &f.ty;
                        Some(quote_spanned! { f.span() => #ty: ArchiveCopy })
                    }
                });
                let field_wheres = quote! { #(#field_wheres,)* };

                quote! {
                    impl<#generic_params> Unarchive<#name<#generic_args>> for Archived<#name<#generic_args>>
                    where
                        #generic_predicates
                        #field_wheres
                    {
                        fn unarchive(&self) -> Self {
                            *self
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<#generic_params> Unarchive<#name<#generic_args>> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                {
                    fn unarchive(&self) -> Self {
                        *self
                    }
                }
            },
        },
        Data::Enum(ref data) => {
            let field_wheres = data.variants.iter().map(|v| match v.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: ArchiveCopy })
                        }
                    });
                    quote! { #(#field_wheres,)* }
                }
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().filter_map(|f| {
                        if f.attrs.iter().any(|a| a.path.is_ident("recursive")) {
                            None
                        } else {
                            let ty = &f.ty;
                            Some(quote_spanned! { f.span() => #ty: ArchiveCopy })
                        }
                    });
                    quote! { #(#field_wheres,)* }
                }
                Fields::Unit => quote! {},
            });
            let field_wheres = quote! { #(#field_wheres)* };

            quote! {
                impl<#generic_params> Unarchive<#name<#generic_args>> for Archived<#name<#generic_args>>
                where
                    #generic_predicates
                    #field_wheres
                {
                    fn unarchive(&self) -> Self {
                        *self
                    }
                }
            }
        }
        Data::Union(_) => {
            return Error::new(input.span(), "Unarchive cannot be derived for unions")
                .to_compile_error()
        }
    };

    quote! {
        const _: () = {
            use rkyv::{Archive, Archived, ArchiveCopy, Unarchive};
            #unarchive_impl
        };
    }
}
