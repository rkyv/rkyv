use crate::attributes::{parse_attributes, Attributes};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    parse_quote, spanned::Spanned, Attribute, Data, DeriveInput, Error, Fields, Ident, Index,
};

pub fn derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = parse_attributes(&input)?;

    if attributes.copy.is_some() {
        derive_archive_copy_impl(input, &attributes)
    } else {
        derive_archive_impl(input, &attributes)
    }
}

fn derive_archive_impl(
    mut input: DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    input.generics.make_where_clause();

    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let archive_derives = attributes
        .derives
        .as_ref()
        .map::<Attribute, _>(|d| parse_quote! { #[#d] });

    let archived = attributes.archived.as_ref().map_or_else(
        || Ident::new(&format!("Archived{}", name), name.span()),
        |value| value.clone(),
    );
    let archived_doc = format!("An archived `{}`", name);

    let resolver = attributes.resolver.as_ref().map_or_else(
        || Ident::new(&format!("{}Resolver", name), name.span()),
        |value| value.clone(),
    );
    let resolver_doc = format!("The resolver for archived `{}`", name);

    let is_strict = cfg!(feature = "strict") || attributes.strict.is_some();
    let strict = is_strict.then::<Attribute, _>(|| parse_quote! { #[repr(C)] });

    let (archive_types, archive_impls) = match input.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let mut archive_where = where_clause.clone();
                    for field in fields
                        .named
                        .iter()
                        .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                    {
                        let ty = &field.ty;
                        archive_where
                            .predicates
                            .push(parse_quote! { #ty: rkyv::Archive });
                    }

                    let resolver_fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #name: rkyv::Resolver<#ty> }
                    });

                    let archived_fields = fields.named.iter().map(|f| {
                        let field_name = f.ident.as_ref();
                        let ty = &f.ty;
                        let vis = &f.vis;
                        let field_doc = format!(
                            "The archived counterpart of `{}::{}`",
                            name,
                            field_name.unwrap()
                        );
                        quote_spanned! { f.span() =>
                            #[doc = #field_doc]
                            #vis #field_name: rkyv::Archived<#ty>
                        }
                    });

                    let resolve_fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! { f.span() =>
                            self.#name.resolve(
                                pos + rkyv::offset_of!(#archived #ty_generics, #name),
                                resolver.#name,
                                rkyv::project_struct!(out: Self::Archived => #name)
                            )
                        }
                    });

                    let mut partial_eq_impl = None;
                    let mut partial_ord_impl = None;
                    if let Some((_, ref compares)) = attributes.compares {
                        for compare in compares {
                            if compare.is_ident("PartialEq") {
                                let mut partial_eq_where = archive_where.clone();
                                for field in fields.named.iter().filter(|f| {
                                    !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds"))
                                }) {
                                    let ty = &field.ty;
                                    partial_eq_where
                                        .predicates
                                        .push(parse_quote! { rkyv::Archived<#ty>: PartialEq<#ty> });
                                }

                                let field_names = fields.named.iter().map(|f| &f.ident);

                                partial_eq_impl = Some(quote! {
                                    impl #impl_generics PartialEq<#archived #ty_generics> for #name #ty_generics #partial_eq_where {
                                        #[inline]
                                        fn eq(&self, other: &#archived #ty_generics) -> bool {
                                            true #(&& other.#field_names.eq(&self.#field_names))*
                                        }
                                    }

                                    impl #impl_generics PartialEq<#name #ty_generics> for #archived #ty_generics #partial_eq_where {
                                        #[inline]
                                        fn eq(&self, other: &#name #ty_generics) -> bool {
                                            other.eq(self)
                                        }
                                    }
                                });
                            } else if compare.is_ident("PartialOrd") {
                                let mut partial_ord_where = archive_where.clone();
                                for field in fields.named.iter().filter(|f| {
                                    !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds"))
                                }) {
                                    let ty = &field.ty;
                                    partial_ord_where.predicates.push(
                                        parse_quote! { rkyv::Archived<#ty>: PartialOrd<#ty> },
                                    );
                                }

                                let field_names = fields.named.iter().map(|f| &f.ident);

                                partial_ord_impl = Some(quote! {
                                    impl #impl_generics PartialOrd<#archived #ty_generics> for #name #ty_generics #partial_ord_where {
                                        #[inline]
                                        fn partial_cmp(&self, other: &#archived #ty_generics) -> Option<::core::cmp::Ordering> {
                                            #(
                                                match other.#field_names.partial_cmp(&self.#field_names) {
                                                    Some(::core::cmp::Ordering::Equal) => (),
                                                    x => return x,
                                                }
                                            )*
                                            Some(::core::cmp::Ordering::Equal)
                                        }
                                    }

                                    impl #impl_generics PartialOrd<#name #ty_generics> for #archived #ty_generics #partial_ord_where {
                                        #[inline]
                                        fn partial_cmp(&self, other: &#name #ty_generics) -> Option<::core::cmp::Ordering> {
                                            other.partial_cmp(self)
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
                            #[doc = #archived_doc]
                            #archive_derives
                            #strict
                            #vis struct #archived #generics #archive_where {
                                #(#archived_fields,)*
                            }

                            #[doc = #resolver_doc]
                            #vis struct #resolver #generics #archive_where {
                                #(#resolver_fields,)*
                            }
                        },
                        quote! {
                            impl #impl_generics rkyv::Archive for #name #ty_generics #archive_where {
                                type Archived = #archived #ty_generics;
                                type Resolver = #resolver #ty_generics;

                                #[allow(clippy::unit_arg)]
                                #[inline]
                                fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                                    #(#resolve_fields;)*
                                }
                            }

                            #partial_eq_impl
                            #partial_ord_impl
                        },
                    )
                }
                Fields::Unnamed(ref fields) => {
                    let mut archive_where = where_clause.clone();
                    for field in fields
                        .unnamed
                        .iter()
                        .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                    {
                        let ty = &field.ty;
                        archive_where
                            .predicates
                            .push(parse_quote! { #ty: rkyv::Archive });
                    }

                    let resolver_fields = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => rkyv::Resolver<#ty> }
                    });

                    let archived_fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let ty = &f.ty;
                        let vis = &f.vis;
                        let field_doc = format!("The archived counterpart of `{}::{}`", name, i);
                        quote_spanned! { f.span() =>
                            #[doc = #field_doc]
                            #vis rkyv::Archived<#ty>
                        }
                    });

                    let resolve_fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let index = Index::from(i);
                        quote_spanned! { f.span() =>
                            self.#index.resolve(
                                pos + rkyv::offset_of!(#archived #ty_generics, #index),
                                resolver.#index,
                                rkyv::project_struct!(out: Self::Archived => #index)
                            )
                        }
                    });

                    let mut partial_eq_impl = None;
                    let mut partial_ord_impl = None;
                    if let Some((_, ref compares)) = attributes.compares {
                        for compare in compares {
                            if compare.is_ident("PartialEq") {
                                let mut partial_eq_where = archive_where.clone();
                                for field in fields.unnamed.iter().filter(|f| {
                                    !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds"))
                                }) {
                                    let ty = &field.ty;
                                    partial_eq_where
                                        .predicates
                                        .push(parse_quote! { rkyv::Archived<#ty>: PartialEq<#ty> });
                                }

                                let field_names = fields
                                    .unnamed
                                    .iter()
                                    .enumerate()
                                    .map(|(i, _)| Index::from(i));

                                partial_eq_impl = Some(quote! {
                                    impl #impl_generics PartialEq<#archived #ty_generics> for #name #ty_generics #partial_eq_where {
                                        #[inline]
                                        fn eq(&self, other: &#archived #ty_generics) -> bool {
                                            true #(&& other.#field_names.eq(&self.#field_names))*
                                        }
                                    }

                                    impl #impl_generics PartialEq<#name #ty_generics> for #archived #ty_generics #partial_eq_where {
                                        #[inline]
                                        fn eq(&self, other: &#name #ty_generics) -> bool {
                                            other.eq(self)
                                        }
                                    }
                                });
                            } else if compare.is_ident("PartialOrd") {
                                let mut partial_ord_where = archive_where.clone();
                                for field in fields.unnamed.iter().filter(|f| {
                                    !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds"))
                                }) {
                                    let ty = &field.ty;
                                    partial_ord_where.predicates.push(
                                        parse_quote! { rkyv::Archived<#ty>: PartialOrd<#ty> },
                                    );
                                }

                                let field_names = fields
                                    .unnamed
                                    .iter()
                                    .enumerate()
                                    .map(|(i, _)| Index::from(i));

                                partial_ord_impl = Some(quote! {
                                    impl #impl_generics PartialOrd<#archived #ty_generics> for #name #ty_generics #partial_ord_where {
                                        #[inline]
                                        fn partial_cmp(&self, other: &#archived #ty_generics) -> Option<::core::cmp::Ordering> {
                                            #(
                                                match other.#field_names.partial_cmp(&self.#field_names) {
                                                    Some(::core::cmp::Ordering::Equal) => (),
                                                    x => return x,
                                                }
                                            )*
                                            Some(::core::cmp::Ordering::Equal)
                                        }
                                    }

                                    impl #impl_generics PartialOrd<#name #ty_generics> for #archived #ty_generics #partial_ord_where {
                                        #[inline]
                                        fn partial_cmp(&self, other: &#name #ty_generics) -> Option<::core::cmp::Ordering> {
                                            other.partial_cmp(self)
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
                            #[doc = #archived_doc]
                            #archive_derives
                            #strict
                            #vis struct #archived #generics (#(#archived_fields,)*) #archive_where;

                            #[doc = #resolver_doc]
                            #vis struct #resolver #generics (#(#resolver_fields,)*) #archive_where;
                        },
                        quote! {
                            impl #impl_generics rkyv::Archive for #name #ty_generics #archive_where {
                                type Archived = #archived #ty_generics;
                                type Resolver = #resolver #ty_generics;

                                #[allow(clippy::unit_arg)]
                                #[inline]
                                fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                                    #(#resolve_fields;)*
                                }
                            }

                            #partial_eq_impl
                            #partial_ord_impl
                        },
                    )
                }
                Fields::Unit => {
                    let mut partial_eq_impl = None;
                    let mut partial_ord_impl = None;
                    if let Some((_, ref compares)) = attributes.compares {
                        for compare in compares {
                            if compare.is_ident("PartialEq") {
                                partial_eq_impl = Some(quote! {
                                    impl #impl_generics PartialEq<#archived #ty_generics> for #name #ty_generics #where_clause {
                                        #[inline]
                                        fn eq(&self, _: &#archived #ty_generics) -> bool {
                                            true
                                        }
                                    }

                                    impl #impl_generics PartialEq<#name #ty_generics> for #archived #ty_generics #where_clause {
                                        #[inline]
                                        fn eq(&self, _: &#name #ty_generics) -> bool {
                                            true
                                        }
                                    }
                                });
                            } else if compare.is_ident("PartialOrd") {
                                partial_ord_impl = Some(quote! {
                                    impl #impl_generics PartialOrd<#archived #ty_generics> for #name #ty_generics #where_clause {
                                        #[inline]
                                        fn partial_cmp(&self, _: &#archived #ty_generics) -> Option<::core::cmp::Ordering> {
                                            Some(::core::cmp::Ordering::Equal)
                                        }
                                    }

                                    impl #impl_generics PartialOrd<#name #ty_generics> for #archived #ty_generics #where_clause {
                                        #[inline]
                                        fn partial_cmp(&self, _:&#name #ty_generics) -> Option<::core::cmp::Ordering> {
                                            Some(::core::cmp::Ordering::Equal)
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
                            #[doc = #archived_doc]
                            #archive_derives
                            #strict
                            #vis struct #archived #generics
                            #where_clause;

                            #[doc = #resolver_doc]
                            #vis struct #resolver #generics
                            #where_clause;
                        },
                        quote! {
                            impl #impl_generics rkyv::Archive for #name #ty_generics #where_clause {
                                type Archived = #archived #ty_generics;
                                type Resolver = #resolver #ty_generics;

                                #[inline]
                                fn resolve(&self, _: usize, _: Self::Resolver, _: &mut MaybeUninit<Self::Archived>) {}
                            }

                            #partial_eq_impl
                            #partial_ord_impl
                        },
                    )
                }
            }
        }
        Data::Enum(ref data) => {
            let mut archive_where = where_clause.clone();
            for variant in data.variants.iter() {
                match variant.fields {
                    Fields::Named(ref fields) => {
                        for field in fields
                            .named
                            .iter()
                            .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                        {
                            let ty = &field.ty;
                            archive_where
                                .predicates
                                .push(parse_quote! { #ty: rkyv::Archive });
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        for field in fields
                            .unnamed
                            .iter()
                            .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                        {
                            let ty = &field.ty;
                            archive_where
                                .predicates
                                .push(parse_quote! { #ty: rkyv::Archive });
                        }
                    }
                    Fields::Unit => (),
                }
            }

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
                            #[allow(dead_code)]
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
                            #[allow(dead_code)]
                            #variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => quote_spanned! { variant.span() =>
                        #[allow(dead_code)]
                        #variant
                    },
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
                        let resolves = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let self_binding = Ident::new(&format!("self_{}", name.as_ref().unwrap().to_string()), name.span());
                            let resolver_binding = Ident::new(&format!("resolver_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote! {
                                #self_binding.resolve(
                                    pos + rkyv::offset_of!(#archived_variant_name #ty_generics, #name),
                                    #resolver_binding,
                                    rkyv::project_struct!(out: #archived_variant_name #ty_generics => #name),
                                )
                            }
                        });
                        quote_spanned! { name.span() =>
                            #resolver::#variant { #(#resolver_bindings,)* } => {
                                match self {
                                    #name::#variant { #(#self_bindings,)* } => {
                                        unsafe {
                                            let out = &mut *out.as_mut_ptr().cast::<MaybeUninit<#archived_variant_name #ty_generics>>();
                                            rkyv::project_struct!(out: #archived_variant_name #ty_generics => __tag: ArchivedTag)
                                                .as_mut_ptr()
                                                .write(ArchivedTag::#variant);
                                            #(#resolves;)*
                                        }
                                    },
                                    #[allow(unreachable_patterns)]
                                    _ => panic!("enum resolver variant does not match value variant"),
                                }
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
                        let resolves = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let index = Index::from(i + 1);
                            let self_binding = Ident::new(&format!("self_{}", i), f.span());
                            let resolver_binding = Ident::new(&format!("resolver_{}", i), f.span());
                            quote! {
                                #self_binding.resolve(
                                    pos + rkyv::offset_of!(#archived_variant_name #ty_generics, #index),
                                    #resolver_binding,
                                    rkyv::project_struct!(out: #archived_variant_name #ty_generics => #index),
                                )
                            }
                        });
                        quote_spanned! { name.span() =>
                            #resolver::#variant( #(#resolver_bindings,)* ) => {
                                match self {
                                    #name::#variant(#(#self_bindings,)*) => {
                                        unsafe {
                                            let out = &mut *out.as_mut_ptr().cast::<MaybeUninit<#archived_variant_name #ty_generics>>();
                                            rkyv::project_struct!(out: #archived_variant_name #ty_generics => 0: ArchivedTag)
                                                .as_mut_ptr()
                                                .write(ArchivedTag::#variant);
                                            #(#resolves;)*
                                        }
                                    },
                                    #[allow(unreachable_patterns)]
                                    _ => panic!("enum resolver variant does not match value variant"),
                                }
                            }
                        }
                    }
                    Fields::Unit => quote_spanned! { name.span() =>
                        #resolver::#variant => {
                            unsafe {
                                out.as_mut_ptr().cast::<ArchivedTag>().write(ArchivedTag::#variant);
                            }
                        }
                    }
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
                            #[allow(dead_code)]
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
                            #[allow(dead_code)]
                            #variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => quote_spanned! { variant.span() =>
                        #[allow(dead_code)]
                        #variant
                    },
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
                            struct #archived_variant_name #generics #archive_where {
                                __tag: ArchivedTag,
                                #(#fields,)*
                                __phantom: PhantomData<#name #ty_generics>,
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
                            struct #archived_variant_name #generics (ArchivedTag, #(#fields,)* PhantomData<#name #ty_generics>) #archive_where;
                        }
                    }
                    Fields::Unit => quote! {}
                }
            });

            let mut partial_eq_impl = None;
            let mut partial_ord_impl = None;
            if let Some((_, ref compares)) = attributes.compares {
                for compare in compares {
                    if compare.is_ident("PartialEq") {
                        let mut partial_eq_where = archive_where.clone();
                        for variant in data.variants.iter() {
                            match variant.fields {
                                Fields::Named(ref fields) => {
                                    for field in fields.named.iter().filter(|f| {
                                        !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds"))
                                    }) {
                                        let ty = &field.ty;
                                        partial_eq_where.predicates.push(
                                            parse_quote! { rkyv::Archived<#ty>: PartialEq<#ty> },
                                        );
                                    }
                                }
                                Fields::Unnamed(ref fields) => {
                                    for field in fields.unnamed.iter().filter(|f| {
                                        !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds"))
                                    }) {
                                        let ty = &field.ty;
                                        partial_eq_where.predicates.push(
                                            parse_quote! { rkyv::Archived<#ty>: PartialEq<#ty> },
                                        );
                                    }
                                }
                                Fields::Unit => (),
                            }
                        }

                        let variant_impls = data.variants.iter().map(|v| {
                            let variant = &v.ident;
                            match v.fields {
                                Fields::Named(ref fields) => {
                                    let field_names = fields.named.iter()
                                        .map(|f| &f.ident)
                                        .collect::<Vec<_>>();
                                    let self_bindings = fields.named.iter().map(|f| {
                                        f.ident.as_ref().map(|ident| {
                                            Ident::new(&format!("self_{}", ident.to_string()), ident.span())
                                        })
                                    }).collect::<Vec<_>>();
                                    let other_bindings = fields.named.iter().map(|f| {
                                        f.ident.as_ref().map(|ident| {
                                            Ident::new(&format!("other_{}", ident.to_string()), ident.span())
                                        })
                                    }).collect::<Vec<_>>();
                                    quote! {
                                        #name::#variant { #(#field_names: #self_bindings,)* } => match other {
                                            #archived::#variant { #(#field_names: #other_bindings,)* } => true #(&& #other_bindings.eq(#self_bindings))*,
                                            #[allow(unreachable_patterns)]
                                            _ => false,
                                        }
                                    }
                                }
                                Fields::Unnamed(ref fields) => {
                                    let self_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                                        Ident::new(&format!("self_{}", i), f.span())
                                    }).collect::<Vec<_>>();
                                    let other_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                                        Ident::new(&format!("other_{}", i), f.span())
                                    }).collect::<Vec<_>>();
                                    quote! {
                                        #name::#variant(#(#self_bindings,)*) => match other {
                                            #archived::#variant(#(#other_bindings,)*) => true #(&& #other_bindings.eq(#self_bindings))*,
                                            #[allow(unreachable_patterns)]
                                            _ => false,
                                        }
                                    }
                                }
                                Fields::Unit => quote! {
                                    #name::#variant => match other {
                                        #archived::#variant => true,
                                        #[allow(unreachable_patterns)]
                                        _ => false,
                                    }
                                }
                            }
                        });

                        partial_eq_impl = Some(quote! {
                            impl #impl_generics PartialEq<#archived #ty_generics> for #name #ty_generics #partial_eq_where {
                                #[inline]
                                fn eq(&self, other: &#archived #ty_generics) -> bool {
                                    match self {
                                        #(#variant_impls,)*
                                    }
                                }
                            }

                            impl #impl_generics PartialEq<#name #ty_generics> for #archived #ty_generics #partial_eq_where {
                                #[inline]
                                fn eq(&self, other: &#name #ty_generics) -> bool {
                                    other.eq(self)
                                }
                            }
                        });
                    } else if compare.is_ident("PartialOrd") {
                        let mut partial_ord_where = archive_where.clone();
                        for variant in data.variants.iter() {
                            match variant.fields {
                                Fields::Named(ref fields) => {
                                    for field in fields.named.iter().filter(|f| {
                                        !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds"))
                                    }) {
                                        let ty = &field.ty;
                                        partial_ord_where.predicates.push(
                                            parse_quote! { rkyv::Archived<#ty>: PartialOrd<#ty> },
                                        );
                                    }
                                }
                                Fields::Unnamed(ref fields) => {
                                    for field in fields.unnamed.iter().filter(|f| {
                                        !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds"))
                                    }) {
                                        let ty = &field.ty;
                                        partial_ord_where.predicates.push(
                                            parse_quote! { rkyv::Archived<#ty>: PartialOrd<#ty> },
                                        );
                                    }
                                }
                                Fields::Unit => (),
                            }
                        }

                        let self_disc = data.variants.iter().enumerate().map(|(i, v)| {
                            let variant = &v.ident;
                            match v.fields {
                                Fields::Named(_) => quote! {
                                    #name::#variant { .. } => #i
                                },
                                Fields::Unnamed(_) => quote! {
                                    #name::#variant ( .. ) => #i
                                },
                                Fields::Unit => quote! {
                                    #name::#variant => #i
                                },
                            }
                        });
                        let other_disc = data.variants.iter().enumerate().map(|(i, v)| {
                            let variant = &v.ident;
                            match v.fields {
                                Fields::Named(_) => quote! {
                                    #archived::#variant { .. } => #i
                                },
                                Fields::Unnamed(_) => quote! {
                                    #archived::#variant ( .. ) => #i
                                },
                                Fields::Unit => quote! {
                                    #archived::#variant => #i
                                },
                            }
                        });

                        let variant_impls = data.variants.iter().map(|v| {
                            let variant = &v.ident;
                            match v.fields {
                                Fields::Named(ref fields) => {
                                    let field_names = fields.named.iter()
                                        .map(|f| &f.ident)
                                        .collect::<Vec<_>>();
                                    let self_bindings = fields.named.iter().map(|f| {
                                        f.ident.as_ref().map(|ident| {
                                            Ident::new(&format!("self_{}", ident.to_string()), ident.span())
                                        })
                                    }).collect::<Vec<_>>();
                                    let other_bindings = fields.named.iter().map(|f| {
                                        f.ident.as_ref().map(|ident| {
                                            Ident::new(&format!("other_{}", ident.to_string()), ident.span())
                                        })
                                    }).collect::<Vec<_>>();
                                    quote! {
                                        #name::#variant { #(#field_names: #self_bindings,)* } => match other {
                                            #archived::#variant { #(#field_names: #other_bindings,)* } => {
                                                #(
                                                    match #other_bindings.partial_cmp(#self_bindings) {
                                                        Some(::core::cmp::Ordering::Equal) => (),
                                                        cmp => return cmp,
                                                    }
                                                )*
                                                Some(::core::cmp::Ordering::Equal)
                                            }
                                            #[allow(unreachable_patterns)]
                                            _ => unsafe { ::core::hint::unreachable_unchecked() },
                                        }
                                    }
                                }
                                Fields::Unnamed(ref fields) => {
                                    let self_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                                        Ident::new(&format!("self_{}", i), f.span())
                                    }).collect::<Vec<_>>();
                                    let other_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                                        Ident::new(&format!("other_{}", i), f.span())
                                    }).collect::<Vec<_>>();
                                    quote! {
                                        #name::#variant(#(#self_bindings,)*) => match other {
                                            #archived::#variant(#(#other_bindings,)*) => {
                                                #(
                                                    match #other_bindings.partial_cmp(#self_bindings) {
                                                        Some(::core::cmp::Ordering::Equal) => (),
                                                        cmp => return cmp,
                                                    }
                                                )*
                                                Some(::core::cmp::Ordering::Equal)
                                            }
                                            #[allow(unreachable_patterns)]
                                            _ => unsafe { ::core::hint::unreachable_unchecked() },
                                        }
                                    }
                                }
                                Fields::Unit => quote! {
                                    #name::#variant => match other {
                                        #archived::#variant => Some(::core::cmp::Ordering::Equal),
                                        #[allow(unreachable_patterns)]
                                        _ => unsafe { ::core::hint::unreachable_unchecked() },
                                    }
                                }
                            }
                        });

                        partial_ord_impl = Some(quote! {
                            impl #impl_generics PartialOrd<#archived #ty_generics> for #name #ty_generics #partial_ord_where {
                                #[inline]
                                fn partial_cmp(&self, other: &#archived #ty_generics) -> Option<::core::cmp::Ordering> {
                                    let self_disc = match self { #(#self_disc,)* };
                                    let other_disc = match other { #(#other_disc,)* };
                                    if self_disc == other_disc {
                                        match self {
                                            #(#variant_impls,)*
                                        }
                                    } else {
                                        self_disc.partial_cmp(&other_disc)
                                    }
                                }
                            }

                            impl #impl_generics PartialOrd<#name #ty_generics> for #archived #ty_generics #partial_ord_where {
                                #[inline]
                                fn partial_cmp(&self, other: &#name #ty_generics) -> Option<::core::cmp::Ordering> {
                                    match other.partial_cmp(self) {
                                        Some(::core::cmp::Ordering::Less) => Some(::core::cmp::Ordering::Greater),
                                        Some(::core::cmp::Ordering::Greater) => Some(::core::cmp::Ordering::Less),
                                        cmp => cmp,
                                    }
                                }
                            }
                        });
                    } else {
                        return Err(Error::new_spanned(compare, "unrecognized compare argument, supported compares are PartialEq (PartialOrd is not supported for enums)"));
                    }
                }
            }

            (
                quote! {
                    #[doc = #archived_doc]
                    #archive_derives
                    #[repr(#archived_repr)]
                    #vis enum #archived #generics #archive_where {
                        #(#archived_variants,)*
                    }

                    #[doc = #resolver_doc]
                    #vis enum #resolver #generics #archive_where {
                        #(#resolver_variants,)*
                    }
                },
                quote! {
                    #[repr(#archived_repr)]
                    enum ArchivedTag {
                        #(#archived_variant_tags,)*
                    }

                    #(#archived_variant_structs)*

                    impl #impl_generics rkyv::Archive for #name #ty_generics #archive_where {
                        type Archived = #archived #ty_generics;
                        type Resolver = #resolver #ty_generics;

                        #[allow(clippy::unit_arg)]
                        #[inline]
                        fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                            match resolver {
                                #(#resolve_arms,)*
                            }
                        }
                    }

                    #partial_eq_impl
                    #partial_ord_impl
                },
            )
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Archive cannot be derived for unions",
            ))
        }
    };

    Ok(quote! {
        #archive_types

        const _: () = {
            use core::{marker::PhantomData, mem::MaybeUninit};

            #archive_impls
        };
    })
}

fn derive_archive_copy_impl(
    mut input: DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    if let Some(ref derives) = attributes.derives {
        return Err(Error::new_spanned(
            derives,
            "derives should be placed on the derived type for archive self derives",
        ));
    }

    if let Some((ref compares, _)) = attributes.compares {
        return Err(Error::new_spanned(
            compares,
            "compares should be placed on the derived type for archive self derives",
        ));
    }

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

    input.generics.make_where_clause();

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let archive_copy_impl = match input.data {
        Data::Struct(ref data) => {
            let mut copy_where = where_clause.clone();
            match data.fields {
                Fields::Named(ref fields) => {
                    for field in fields.named.iter() {
                        let ty = &field.ty;
                        copy_where
                            .predicates
                            .push(parse_quote! { #ty: ArchiveCopy });
                    }
                }
                Fields::Unnamed(ref fields) => {
                    for field in fields.unnamed.iter() {
                        let ty = &field.ty;
                        copy_where
                            .predicates
                            .push(parse_quote! { #ty: ArchiveCopy });
                    }
                }
                Fields::Unit => (),
            }

            quote! {
                unsafe impl #impl_generics ArchiveCopy for #name #ty_generics #copy_where {}

                impl #impl_generics Archive for #name #ty_generics #copy_where {
                    type Archived = Self;
                    type Resolver = ();

                    #[inline]
                    fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                        unsafe {
                            out.as_mut_ptr().write(*self);
                        }
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

            let mut copy_where = where_clause.clone();
            for variant in data.variants.iter() {
                match variant.fields {
                    Fields::Named(ref fields) => {
                        for field in fields.named.iter() {
                            let ty = &field.ty;
                            copy_where
                                .predicates
                                .push(parse_quote! { #ty: ArchiveCopy });
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        for field in fields.unnamed.iter() {
                            let ty = &field.ty;
                            copy_where
                                .predicates
                                .push(parse_quote! { #ty: ArchiveCopy });
                        }
                    }
                    Fields::Unit => (),
                }
            }

            quote! {
                unsafe impl #impl_generics ArchiveCopy for #name #ty_generics #copy_where {}

                impl #impl_generics Archive for #name #ty_generics #copy_where {
                    type Archived = Self;
                    type Resolver = ();

                    #[inline]
                    fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                        unsafe {
                            out.as_mut_ptr().write(*self);
                        }
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
            use core::mem::MaybeUninit;
            use rkyv::{
                Archive,
                ArchiveCopy,
                Serialize,
            };

            #archive_copy_impl
        };
    })
}
