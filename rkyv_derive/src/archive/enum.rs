use core::fmt::Display;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse_quote, spanned::Spanned as _, DataEnum, Error, Field, Fields,
    Generics, Ident, Index, Member, Path,
};

use crate::{
    archive::{
        archived_doc, printing::Printing, resolver_doc, resolver_variant_doc,
        variant_doc,
    },
    attributes::{Attributes, FieldAttributes},
    util::{strip_generics_from_path, strip_raw},
};

pub fn impl_enum(
    printing: &Printing,
    generics: &Generics,
    attributes: &Attributes,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        name,
        archived_type,
        resolver_name,
        ..
    } = printing;

    if data.variants.len() > 256 {
        return Err(Error::new_spanned(
            &printing.name,
            "enums with more than 256 variants cannot derive Archive",
        ));
    }

    let mut public = TokenStream::new();
    let mut private = TokenStream::new();

    if attributes.as_type.is_none() {
        public.extend(generate_archived_type(
            printing, attributes, generics, data,
        )?);

        private.extend(generate_niching_impls(
            printing, attributes, generics, data,
        )?);
    }

    public.extend(generate_resolver_type(
        printing, attributes, generics, data,
    )?);

    let archived_variant_tags = data.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let (eq, expr) = variant
            .discriminant
            .as_ref()
            .map(|(eq, expr)| (eq, expr))
            .unzip();
        quote! { #ident #eq #expr }
    });
    private.extend(quote! {
        #[derive(PartialEq, PartialOrd)]
        #[repr(u8)]
        enum ArchivedTag {
            #(#archived_variant_tags,)*
        }
    });

    private.extend(generate_variant_structs(
        printing, attributes, generics, data,
    )?);

    if let Some(ref compares) = attributes.compares {
        for compare in compares {
            if compare.is_ident("PartialEq") {
                public.extend(generate_partial_eq_impl(
                    printing, attributes, generics, data,
                )?);
            } else if compare.is_ident("PartialOrd") {
                private.extend(generate_partial_ord_impl(
                    printing, attributes, generics, data,
                )?);
            } else {
                return Err(Error::new_spanned(
                    compare,
                    "unrecognized compare argument, supported compares are \
                     PartialEq (PartialOrd is not supported for enums)",
                ));
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let archive_impl = if let Some(ref remote) = attributes.remote {
        let resolve_arms = generate_resolve_arms(
            printing,
            attributes,
            generics,
            data,
            &strip_generics_from_path(remote.clone()),
        )?;

        quote! {
            impl #impl_generics #rkyv_path::with::ArchiveWith<#remote>
                for #name #ty_generics
            #where_clause
            {
                type Archived = #archived_type;
                type Resolver = #resolver_name #ty_generics;

                // Some resolvers will be (), this allow is to prevent clippy
                // from complaining
                #[allow(clippy::unit_arg)]
                fn resolve_with(
                    field: &#remote,
                    resolver: Self::Resolver,
                    out: #rkyv_path::Place<Self::Archived>,
                ) {
                    let __this = field;
                    match resolver {
                        #resolve_arms
                    }
                }
            }
        }
    } else {
        let resolve_arms = generate_resolve_arms(
            printing,
            attributes,
            generics,
            data,
            &parse_quote!(#name),
        )?;

        quote! {
            impl #impl_generics #rkyv_path::Archive for #name #ty_generics
            #where_clause
            {
                type Archived = #archived_type;
                type Resolver = #resolver_name #ty_generics;

                // Some resolvers will be (), this allow is to prevent clippy
                // from complaining
                #[allow(clippy::unit_arg)]
                fn resolve(
                    &self,
                    resolver: Self::Resolver,
                    out: #rkyv_path::Place<Self::Archived>,
                ) {
                    let __this = self;
                    match resolver {
                        #resolve_arms
                    }
                }
            }
        }
    };

    Ok(quote! {
        #public

        const _: () = {
            #private

            #archive_impl
        };
    })
}

fn generate_archived_type(
    printing: &Printing,
    attributes: &Attributes,
    generics: &Generics,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        vis,
        name,
        archived_metas,
        archived_name,
        ..
    } = printing;

    let mut archived_variants = TokenStream::new();
    for variant in &data.variants {
        let variant_name = &variant.ident;
        let (eq, expr) = variant
            .discriminant
            .as_ref()
            .map(|(eq, expr)| (eq, expr))
            .unzip();

        let variant_doc = variant_doc(name, variant_name);

        let mut variant_fields = TokenStream::new();
        for (i, field) in variant.fields.iter().enumerate() {
            let Field {
                vis,
                ident,
                colon_token,
                ..
            } = field;
            let field_attrs = FieldAttributes::parse(attributes, field)?;

            let field_doc = format!(
                "The archived counterpart of [`{}::{}::{}`]",
                name,
                variant_name,
                ident.as_ref().map_or_else(
                    || &i as &dyn Display,
                    |name| name as &dyn Display
                )
            );

            let field_ty = field_attrs.archived(rkyv_path, field);
            let field_metas = field_attrs.metas();
            variant_fields.extend(quote! {
                #[doc = #field_doc]
                #field_metas
                #vis #ident #colon_token #field_ty,
            });
        }

        archived_variants.extend(match variant.fields {
            Fields::Named(_) => quote! {
                #[doc = #variant_doc]
                #[allow(dead_code)]
                #variant_name {
                    #variant_fields
                } #eq #expr,
            },
            Fields::Unnamed(_) => quote! {
                #[doc = #variant_doc]
                #[allow(dead_code)]
                #variant_name(#variant_fields) #eq #expr,
            },
            Fields::Unit => quote! {
                #[doc = #variant_doc]
                #[allow(dead_code)]
                #variant_name #eq #expr,
            },
        });
    }

    let where_clause = &generics.where_clause;
    let archived_doc = archived_doc(name);
    Ok(quote! {
        #[automatically_derived]
        #[doc = #archived_doc]
        #(#[#archived_metas])*
        #[repr(u8)]
        #vis enum #archived_name #generics #where_clause {
            #archived_variants
        }
    })
}

fn generate_resolver_type(
    printing: &Printing,
    attributes: &Attributes,
    generics: &Generics,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        vis,
        name,
        resolver_name,
        ..
    } = printing;

    let mut resolver_variants = TokenStream::new();
    for variant in &data.variants {
        let variant_name = &variant.ident;

        let variant_doc = resolver_variant_doc(name, variant_name);

        let mut variant_fields = TokenStream::new();
        for field in variant.fields.iter() {
            let Field {
                ident, colon_token, ..
            } = field;
            let field_attrs = FieldAttributes::parse(attributes, field)?;

            let field_ty = field_attrs.resolver(rkyv_path, field);
            variant_fields.extend(quote! {
                #ident #colon_token #field_ty,
            });
        }

        resolver_variants.extend(match variant.fields {
            Fields::Named(_) => quote! {
                #[doc = #variant_doc]
                #[allow(dead_code)]
                #variant_name {
                    #variant_fields
                },
            },
            Fields::Unnamed(_) => quote! {
                #[doc = #variant_doc]
                #[allow(dead_code)]
                #variant_name(#variant_fields),
            },
            Fields::Unit => quote! {
                #[doc = #variant_doc]
                #[allow(dead_code)]
                #variant_name,
            },
        });
    }

    let where_clause = &generics.where_clause;
    let resolver_doc = resolver_doc(name);
    Ok(quote! {
        #[automatically_derived]
        #[doc = #resolver_doc]
        #vis enum #resolver_name #generics #where_clause {
            #resolver_variants
        }
    })
}

fn generate_resolve_arms(
    printing: &Printing,
    attributes: &Attributes,
    generics: &Generics,
    data: &DataEnum,
    name: &Path,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        resolver_name,
        ..
    } = printing;
    let (_, ty_generics, _) = generics.split_for_impl();

    let mut result = TokenStream::new();
    for variant in &data.variants {
        let variant_name = &variant.ident;
        let archived_variant_name =
            format_ident!("ArchivedVariant{}", strip_raw(variant_name),);

        let members = variant
            .fields
            .members()
            .map(|member| match member {
                Member::Named(_) => member,
                Member::Unnamed(index) => Member::Unnamed(Index {
                    index: index.index + 1,
                    span: index.span,
                }),
            })
            .collect::<Vec<_>>();

        let (self_bindings, resolver_bindings) = variant
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                (
                    Ident::new(&format!("self_{i}"), field.span()),
                    Ident::new(&format!("resolver_{i}"), field.span()),
                )
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        let resolves = variant
            .fields
            .iter()
            .map(|f| {
                let field_attrs = FieldAttributes::parse(attributes, f)?;
                Ok(field_attrs.resolve(rkyv_path, f))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        match variant.fields {
            Fields::Named(_) => result.extend(quote! {
                #resolver_name::#variant_name {
                    #(#members: #resolver_bindings,)*
                } => {
                    match __this {
                        #name::#variant_name {
                            #(#members: #self_bindings,)*..
                        } => {
                            let out = unsafe {
                                out.cast_unchecked::<
                                    #archived_variant_name #ty_generics
                                >()
                            };
                            let tag_ptr = unsafe {
                                ::core::ptr::addr_of_mut!(
                                    (*out.ptr()).__tag
                                )
                            };
                            unsafe {
                                tag_ptr.write(ArchivedTag::#variant_name);
                            }
                            #(
                                let field_ptr = unsafe {
                                    ::core::ptr::addr_of_mut!(
                                        (*out.ptr()).#members
                                    )
                                };
                                let out_field = unsafe {
                                    #rkyv_path::Place::from_field_unchecked(
                                        out,
                                        field_ptr,
                                    )
                                };
                                #resolves(
                                    #self_bindings,
                                    #resolver_bindings,
                                    out_field,
                                );
                            )*
                        },
                        #[allow(unreachable_patterns)]
                        _ => unsafe {
                            ::core::hint::unreachable_unchecked()
                        },
                    }
                }
            }),
            Fields::Unnamed(_) => result.extend(quote! {
                #resolver_name::#variant_name( #(#resolver_bindings,)* ) => {
                    match __this {
                        #name::#variant_name(#(#self_bindings,)* ..) => {
                            let out = unsafe {
                                out.cast_unchecked::<
                                    #archived_variant_name #ty_generics
                                >()
                            };
                            let tag_ptr = unsafe {
                                ::core::ptr::addr_of_mut!((*out.ptr()).0)
                            };
                            unsafe {
                                tag_ptr.write(ArchivedTag::#variant_name);
                            }
                            #(
                                let field_ptr = unsafe {
                                    ::core::ptr::addr_of_mut!(
                                        (*out.ptr()).#members
                                    )
                                };
                                let out_field = unsafe {
                                    #rkyv_path::Place::from_field_unchecked(
                                        out,
                                        field_ptr,
                                    )
                                };
                                #resolves(
                                    #self_bindings,
                                    #resolver_bindings,
                                    out_field,
                                );
                            )*
                        },
                        #[allow(unreachable_patterns)]
                        _ => unsafe {
                            ::core::hint::unreachable_unchecked()
                        },
                    }
                }
            }),
            Fields::Unit => result.extend(quote! {
                #resolver_name::#variant_name => {
                    let out = unsafe {
                        out.cast_unchecked::<ArchivedTag>()
                    };
                    // SAFETY: `ArchivedTag` is `repr(u8)` and so is always
                    // initialized.
                    unsafe {
                        out.write_unchecked(ArchivedTag::#variant_name);
                    }
                }
            }),
        }
    }

    Ok(result)
}

fn generate_variant_structs(
    printing: &Printing,
    attributes: &Attributes,
    generics: &Generics,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path, name, ..
    } = printing;
    let (_, ty_generics, _) = generics.split_for_impl();
    let where_clause = &generics.where_clause;

    let mut result = TokenStream::new();
    for variant in &data.variants {
        let archived_variant_name =
            format_ident!("ArchivedVariant{}", strip_raw(&variant.ident),);

        let mut archived_fields = TokenStream::new();
        for field in variant.fields.iter() {
            let field_attrs = FieldAttributes::parse(attributes, field)?;
            let archived = field_attrs.archived(rkyv_path, field);

            let Field {
                ident, colon_token, ..
            } = field;

            archived_fields.extend(quote! {
                #ident #colon_token #archived,
            });
        }

        match variant.fields {
            Fields::Named(_) => result.extend(quote! {
                #[repr(C)]
                struct #archived_variant_name #generics #where_clause {
                    __tag: ArchivedTag,
                    #archived_fields
                    __phantom: ::core::marker::PhantomData<
                        #name #ty_generics
                    >,
                }
            }),
            Fields::Unnamed(_) => result.extend(quote! {
                #[repr(C)]
                struct #archived_variant_name #generics (
                    ArchivedTag,
                    #archived_fields
                    ::core::marker::PhantomData<#name #ty_generics>,
                ) #where_clause;
            }),
            Fields::Unit => (),
        }
    }

    Ok(result)
}

fn generate_partial_eq_impl(
    printing: &Printing,
    attributes: &Attributes,
    generics: &Generics,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let Printing {
        archived_name,
        archived_type,
        name,
        ..
    } = printing;
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let mut where_clause = generics.where_clause.clone().unwrap();

    for field in data.variants.iter().flat_map(|v| v.fields.iter()) {
        let field_attrs = FieldAttributes::parse(attributes, field)?;
        if field_attrs.omit_bounds.is_none() {
            let field_attrs = FieldAttributes::parse(attributes, field)?;
            let ty = &field.ty;
            let archived = field_attrs.archived(&printing.rkyv_path, field);
            where_clause
                .predicates
                .push(parse_quote! { #archived: PartialEq<#ty> });
        }
    }

    let variant_impls = data.variants.iter().map(|v| {
        let variant = &v.ident;

        let (self_fields, other_fields) = v
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    Ident::new(&format!("self_{i}"), f.span()),
                    Ident::new(&format!("other_{i}"), f.span()),
                )
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        match v.fields {
            Fields::Named(ref fields) => {
                let field_names =
                    fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();

                quote! {
                    #name::#variant {
                        #(#field_names: #self_fields,)*
                    } => match other {
                        #archived_name::#variant {
                            #(#field_names: #other_fields,)*
                        } => true #(&& #other_fields.eq(#self_fields))*,
                        #[allow(unreachable_patterns)]
                        _ => false,
                    }
                }
            }
            Fields::Unnamed(_) => {
                quote! {
                    #name::#variant(#(#self_fields,)*) => match other {
                        #archived_name::#variant(#(#other_fields,)*) => {
                            true #(&& #other_fields.eq(#self_fields))*
                        }
                        #[allow(unreachable_patterns)]
                        _ => false,
                    }
                }
            }
            Fields::Unit => quote! {
                #name::#variant => match other {
                    #archived_name::#variant => true,
                    #[allow(unreachable_patterns)]
                    _ => false,
                }
            },
        }
    });

    Ok(quote! {
        impl #impl_generics PartialEq<#archived_type> for #name #ty_generics
        #where_clause
        {
            fn eq(&self, other: &#archived_type) -> bool {
                match self {
                    #(#variant_impls,)*
                }
            }
        }

        impl #impl_generics PartialEq<#name #ty_generics> for #archived_type
        #where_clause
        {
            fn eq(&self, other: &#name #ty_generics) -> bool {
                other.eq(self)
            }
        }
    })
}

fn generate_partial_ord_impl(
    printing: &Printing,
    attributes: &Attributes,
    generics: &Generics,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let Printing {
        archived_name,
        archived_type,
        name,
        ..
    } = printing;
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let mut where_clause = generics.where_clause.clone().unwrap();

    for field in data.variants.iter().flat_map(|v| v.fields.iter()) {
        let field_attrs = FieldAttributes::parse(attributes, field)?;
        if field_attrs.omit_bounds.is_none() {
            let ty = &field.ty;
            let archived = field_attrs.archived(&printing.rkyv_path, field);
            where_clause
                .predicates
                .push(parse_quote! { #archived: PartialOrd<#ty> });
        }
    }

    let self_disc = data.variants.iter().map(|v| {
        let variant = &v.ident;
        match v.fields {
            Fields::Named(_) => quote! {
                #name::#variant { .. } => ArchivedTag::#variant
            },
            Fields::Unnamed(_) => quote! {
                #name::#variant ( .. ) => ArchivedTag::#variant
            },
            Fields::Unit => quote! {
                #name::#variant => ArchivedTag::#variant
            },
        }
    });
    let other_disc = data.variants.iter().map(|v| {
        let variant = &v.ident;
        match v.fields {
            Fields::Named(_) => quote! {
                #archived_name::#variant { .. } => ArchivedTag::#variant
            },
            Fields::Unnamed(_) => quote! {
                #archived_name::#variant ( .. ) => ArchivedTag::#variant
            },
            Fields::Unit => quote! {
                #archived_name::#variant => ArchivedTag::#variant
            },
        }
    });

    let variant_impls = data.variants.iter().map(|v| {
        let variant = &v.ident;

        let (self_fields, other_fields) = v
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    Ident::new(&format!("self_{i}"), f.span()),
                    Ident::new(&format!("other_{i}"), f.span()),
                )
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        match v.fields {
            Fields::Named(ref fields) => {
                let field_names =
                    fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();

                quote! {
                    #name::#variant {
                        #(#field_names: #self_fields,)*
                    } => match other {
                        #archived_name::#variant {
                            #(#field_names: #other_fields,)*
                        } => {
                            #(
                                match #other_fields.partial_cmp(#self_fields) {
                                    Some(::core::cmp::Ordering::Equal) => (),
                                    cmp => return cmp.map(
                                        ::core::cmp::Ordering::reverse
                                    ),
                                }
                            )*
                            Some(::core::cmp::Ordering::Equal)
                        }
                        #[allow(unreachable_patterns)]
                        _ => unsafe { ::core::hint::unreachable_unchecked() },
                    }
                }
            }
            Fields::Unnamed(_) => {
                quote! {
                    #name::#variant(#(#self_fields,)*) => match other {
                        #archived_name::#variant(#(#other_fields,)*) => {
                            #(
                                match #other_fields.partial_cmp(#self_fields) {
                                    Some(::core::cmp::Ordering::Equal) => (),
                                    cmp => return cmp.map(
                                        ::core::cmp::Ordering::reverse
                                    ),
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
                    #archived_name::#variant => {
                        Some(::core::cmp::Ordering::Equal)
                    }
                    #[allow(unreachable_patterns)]
                    _ => unsafe { ::core::hint::unreachable_unchecked() },
                }
            },
        }
    });

    Ok(quote! {
        impl #impl_generics PartialOrd<#archived_type> for #name #ty_generics
        #where_clause
        {
            fn partial_cmp(
                &self,
                other: &#archived_type,
            ) -> Option<::core::cmp::Ordering> {
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

        impl #impl_generics PartialOrd<#name #ty_generics> for #archived_type
        #where_clause
        {
            fn partial_cmp(
                &self,
                other: &#name #ty_generics,
            ) -> Option<::core::cmp::Ordering> {
                match other.partial_cmp(self) {
                    Some(::core::cmp::Ordering::Less) => {
                        Some(::core::cmp::Ordering::Greater)
                    }
                    Some(::core::cmp::Ordering::Greater) => {
                        Some(::core::cmp::Ordering::Less)
                    }
                    cmp => cmp,
                }
            }
        }
    })
}

fn generate_niching_impls(
    printing: &Printing,
    attributes: &Attributes,
    generics: &Generics,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        archived_type,
        ..
    } = printing;

    let (impl_generics, ty_generics, ..) = generics.split_for_impl();

    let mut result = TokenStream::new();

    let mut niches = Vec::new();

    for variant in data.variants.iter() {
        let variant_name = &variant.ident;
        let archived_variant_name =
            format_ident!("ArchivedVariant{}", strip_raw(variant_name));

        for (i, field) in variant.fields.iter().enumerate() {
            let field_attrs = FieldAttributes::parse(attributes, field)?;
            let archived_field = field_attrs.archived(rkyv_path, field);

            for niche in field_attrs.niches {
                let niche_tokens = niche.to_tokens(rkyv_path);

                // Best-effort attempt at improving the error message if the
                // same `Niching` implementor type is being used multiple times.
                // Otherwise, the compiler will inform about conflicting impls
                // which are not entirely unreasonable but may appear slightly
                // cryptic.
                if niches.contains(&niche) {
                    return Err(Error::new_spanned(
                        niche_tokens,
                        "each niching type may be used at most once",
                    ));
                }

                let field_member = if let Some(ref name) = field.ident {
                    Member::Named(name.clone())
                } else {
                    Member::Unnamed(Index::from(i + 1))
                };

                let tag_member = if field.ident.is_some() {
                    Member::Named(Ident::new("__tag", Span::call_site()))
                } else {
                    Member::Unnamed(Index::from(0))
                };

                let field_niche = quote! {
                    <#niche_tokens as #rkyv_path::niche::niching::Niching<
                        #archived_field>
                    >
                };

                result.extend(quote! {
                    #[automatically_derived]
                    impl #impl_generics
                        #rkyv_path::niche::niching::Niching<#archived_type>
                    for #niche_tokens {
                        unsafe fn is_niched(niched: *const #archived_type)
                            -> bool
                        {
                            let variant = niched
                                .cast::<#archived_variant_name #ty_generics>();
                            let tag = unsafe {
                                ::core::ptr::addr_of!((*variant).#tag_member)
                            };
                            if unsafe { &*tag != &ArchivedTag::#variant_name } {
                                return false;
                            }
                            let field = unsafe {
                                ::core::ptr::addr_of!((*variant).#field_member)
                            };
                            unsafe { #field_niche::is_niched(field) }
                        }

                        fn resolve_niched(
                            out: #rkyv_path::Place<#archived_type>
                        ) {
                            let out = unsafe {
                                out.cast_unchecked::<
                                    #archived_variant_name #ty_generics
                                >()
                            };
                            let tag_ptr = unsafe {
                                ::core::ptr::addr_of_mut!(
                                    (*out.ptr()).#tag_member
                                )
                            };
                            unsafe {
                                tag_ptr.write(ArchivedTag::#variant_name);
                            }
                            let field_ptr = unsafe {
                                ::core::ptr::addr_of_mut!(
                                    (*out.ptr()).#field_member
                                )
                            };
                            let out_field = unsafe {
                                #rkyv_path::Place::from_field_unchecked(
                                    out, field_ptr,
                                )
                            };
                            #field_niche::resolve_niched(out_field);
                        }
                    }
                });

                niches.push(niche);
            }
        }
    }

    let mut iter = niches.iter();

    while let Some(niche1) = iter.next() {
        let niche1_tokens = niche1.to_tokens(rkyv_path);
        for niche2 in iter.clone() {
            let niche2_tokens = niche2.to_tokens(rkyv_path);
            result.extend(quote! {
                #[automatically_derived]
                unsafe impl #impl_generics
                    #rkyv_path::niche::niching::SharedNiching<
                        #niche1_tokens, #niche2_tokens
                    >
                for #archived_type {}

                #[automatically_derived]
                unsafe impl #impl_generics
                    #rkyv_path::niche::niching::SharedNiching<
                        #niche2_tokens, #niche1_tokens
                    >
                for #archived_type {}
            });
        }
    }

    Ok(result)
}
