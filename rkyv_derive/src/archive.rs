mod printing;
mod r#struct;

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{
    parse_quote, spanned::Spanned, Data, DeriveInput, Error, Field, Fields,
    Ident, Index, Meta,
};

use crate::{
    attributes::Attributes,
    util::{
        archive_bound, archived, archived_doc, is_not_omitted, resolve,
        resolver, resolver_doc, strip_raw,
    },
};

pub fn derive(input: &mut DeriveInput) -> Result<TokenStream, Error> {
    let attributes = Attributes::parse(input)?;
    derive_archive_impl(input, &attributes)
}

fn field_archive_attrs(
    field: &Field,
) -> impl '_ + Iterator<Item = &TokenStream> {
    field.attrs.iter().filter_map(|attr| {
        if let Meta::List(list) = &attr.meta {
            if list.path.is_ident("archive_attr") {
                Some(&list.tokens)
            } else {
                None
            }
        } else {
            None
        }
    })
}

fn derive_archive_impl(
    input: &mut DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    let where_clause = input.generics.make_where_clause();
    if let Some(ref bounds) = attributes.archive_bounds {
        for bound in bounds {
            where_clause.predicates.push(bound.clone());
        }
    }

    let printing = printing::Printing::new(input, attributes)?;

    let (archive_types, archive_impls) = match input.data {
        Data::Struct(_) => r#struct::impl_struct(input, attributes, &printing)?,
        Data::Enum(ref data) => {
            let name = &input.ident;
            let vis = &input.vis;
            let generics = &input.generics;

            let rkyv_path = &printing.rkyv_path;

            let archived_name = &printing.archived_name;
            let archived_type = &printing.archived_type;
            let resolver_name = &printing.resolver_name;

            let (impl_generics, ty_generics, where_clause) =
                input.generics.split_for_impl();
            let where_clause = where_clause.unwrap();

            let mut archive_where = where_clause.clone();
            for variant in data.variants.iter() {
                match variant.fields {
                    Fields::Named(ref fields) => {
                        for field in fields.named.iter().filter(is_not_omitted)
                        {
                            archive_where
                                .predicates
                                .push(archive_bound(rkyv_path, field)?);
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        for field in
                            fields.unnamed.iter().filter(is_not_omitted)
                        {
                            archive_where
                                .predicates
                                .push(archive_bound(rkyv_path, field)?);
                        }
                    }
                    Fields::Unit => (),
                }
            }

            let resolver_variants = data
                .variants
                .iter()
                .map(|v| -> Result<TokenStream, Error> {
                    let variant = &v.ident;
                    match v.fields {
                        Fields::Named(ref fields) => {
                            let fields = fields
                                .named
                                .iter()
                                .map(|f| -> Result<TokenStream, Error> {
                                    let field_name = f.ident.as_ref();
                                    let resolver = resolver(rkyv_path, f)?;
                                    let field_doc = format!(
                                        "The resolver for [`{}::{}::{}`]",
                                        name,
                                        variant,
                                        field_name.unwrap(),
                                    );
                                    Ok(quote! {
                                        #[doc = #field_doc]
                                        #field_name: #resolver
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            let variant_doc = format!(
                                "The resolver for [`{}::{}`]",
                                name, variant
                            );
                            Ok(quote! {
                                #[doc = #variant_doc]
                                #[allow(dead_code)]
                                #variant {
                                    #(#fields,)*
                                }
                            })
                        }
                        Fields::Unnamed(ref fields) => {
                            let fields = fields
                                .unnamed
                                .iter()
                                .enumerate()
                                .map(|(i, f)| -> Result<TokenStream, Error> {
                                    let resolver = resolver(rkyv_path, f)?;
                                    let field_doc = format!(
                                        "The resolver for [`{}::{}::{}`]",
                                        name, variant, i
                                    );
                                    Ok(quote! {
                                        #[doc = #field_doc]
                                        #resolver
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            let variant_doc = format!(
                                "The resolver for [`{}::{}`]",
                                name, variant
                            );
                            Ok(quote! {
                                #[doc = #variant_doc]
                                #[allow(dead_code)]
                                #variant(#(#fields,)*)
                            })
                        }
                        Fields::Unit => {
                            let variant_doc = format!(
                                "The resolver for [`{}::{}`]",
                                name, variant
                            );
                            Ok(quote! {
                                #[doc = #variant_doc]
                                #[allow(dead_code)]
                                #variant
                            })
                        }
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

            let resolve_arms = data.variants.iter().map(|v| -> Result<TokenStream, Error> {
                let variant = &v.ident;
                let archived_variant_name = Ident::new(&format!("ArchivedVariant{}", strip_raw(variant)), v.span());
                match v.fields {
                    Fields::Named(ref fields) => {
                        let self_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("self_{}", strip_raw(name.as_ref().unwrap())), name.span());
                            quote! { #name: #binding }
                        });
                        let resolver_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("resolver_{}", strip_raw(name.as_ref().unwrap())), name.span());
                            quote! { #name: #binding }
                        });
                        let resolves = fields.named.iter().map(|f| -> Result<TokenStream, Error> {
                            let name = &f.ident;
                            let self_binding = Ident::new(&format!("self_{}", strip_raw(name.as_ref().unwrap())), name.span());
                            let resolver_binding = Ident::new(&format!("resolver_{}", strip_raw(name.as_ref().unwrap())), name.span());
                            let resolve = resolve(rkyv_path, f)?;
                            Ok(quote! {
                                let (fp, fo) = out_field!(out.#name);
                                #resolve(#self_binding, pos + fp, #resolver_binding, fo);
                            })
                        }).collect::<Result<Vec<_>, _>>()?;
                        Ok(quote! {
                            #resolver_name::#variant { #(#resolver_bindings,)* } => {
                                match self {
                                    #name::#variant { #(#self_bindings,)* } => {
                                        let out = out.cast::<#archived_variant_name #ty_generics>();
                                        ::core::ptr::addr_of_mut!((*out).__tag)
                                            .write(ArchivedTag::#variant);
                                        #(#resolves)*
                                    },
                                    #[allow(unreachable_patterns)]
                                    _ => ::core::hint::unreachable_unchecked(),
                                }
                            }
                        })
                    }
                    Fields::Unnamed(ref fields) => {
                        let self_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("self_{}", i), f.span());
                            quote! { #name }
                        });
                        let resolver_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("resolver_{}", i), f.span());
                            quote! { #name }
                        });
                        let resolves = fields.unnamed.iter().enumerate().map(|(i, f)| -> Result<TokenStream, Error> {
                            let index = Index::from(i + 1);
                            let self_binding = Ident::new(&format!("self_{}", i), f.span());
                            let resolver_binding = Ident::new(&format!("resolver_{}", i), f.span());
                            let resolve = resolve(rkyv_path, f)?;
                            Ok(quote! {
                                let (fp, fo) = out_field!(out.#index);
                                #resolve(#self_binding, pos + fp, #resolver_binding, fo);
                            })
                        }).collect::<Result<Vec<_>, _>>()?;
                        Ok(quote! {
                            #resolver_name::#variant( #(#resolver_bindings,)* ) => {
                                match self {
                                    #name::#variant(#(#self_bindings,)*) => {
                                        let out = out.cast::<#archived_variant_name #ty_generics>();
                                        ::core::ptr::addr_of_mut!((*out).0).write(ArchivedTag::#variant);
                                        #(#resolves)*
                                    },
                                    #[allow(unreachable_patterns)]
                                    _ => ::core::hint::unreachable_unchecked(),
                                }
                            }
                        })
                    }
                    Fields::Unit => Ok(quote! {
                        #resolver_name::#variant => {
                            out.cast::<ArchivedTag>().write(ArchivedTag::#variant);
                        }
                    })
                }
            }).collect::<Result<Vec<_>, _>>()?;

            let repr = match data.variants.len() as u128 {
                0..=255 => quote! { #[repr(u8)] },
                256..=65_535 => quote! { #[repr(u16)] },
                65_536..=4_294_967_295 => quote! { #[repr(u32)] },
                4_294_967_296..=18_446_744_073_709_551_615 => {
                    quote! { #[repr(u64)] }
                }
                _ => quote! { #[repr(u128)] },
            };

            let archived_def = if attributes.archive_as.is_none() {
                let archived_variants = data
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(i, v)| -> Result<TokenStream, Error> {
                        let variant = &v.ident;
                        let discriminant = Literal::usize_unsuffixed(i);
                        match v.fields {
                            Fields::Named(ref fields) => {
                                let fields = fields
                                    .named
                                    .iter()
                                    .map(|f| -> Result<TokenStream, Error> {
                                        let field_name = f.ident.as_ref();
                                        let vis = &f.vis;
                                        let field_doc = format!(
                                            "The archived counterpart of \
                                             [`{}::{}::{}`]",
                                            name,
                                            variant,
                                            field_name.unwrap(),
                                        );
                                        let archive_attrs =
                                            field_archive_attrs(f);
                                        let archived = archived(rkyv_path, f)?;
                                        Ok(quote! {
                                            #[doc = #field_doc]
                                            #(#[#archive_attrs])*
                                            #vis #field_name: #archived
                                        })
                                    })
                                    .collect::<Result<Vec<_>, _>>()?;
                                let variant_doc = format!(
                                    "The archived counterpart of [`{}::{}`]",
                                    name, variant
                                );
                                Ok(quote! {
                                    #[doc = #variant_doc]
                                    #[allow(dead_code)]
                                    #variant {
                                        #(#fields,)*
                                    } = #discriminant
                                })
                            }
                            Fields::Unnamed(ref fields) => {
                                let fields = fields
                                    .unnamed
                                    .iter()
                                    .enumerate()
                                    .map(
                                        |(i, f)| -> Result<TokenStream, Error> {
                                            let vis = &f.vis;
                                            let field_doc = format!(
                                                "The archived counterpart of \
                                                 [`{}::{}::{}`]",
                                                name, variant, i,
                                            );
                                            let archive_attrs =
                                                field_archive_attrs(f);
                                            let archived =
                                                archived(rkyv_path, f)?;
                                            Ok(quote! {
                                                #[doc = #field_doc]
                                                #(#[#archive_attrs])*
                                                #vis #archived
                                            })
                                        },
                                    )
                                    .collect::<Result<Vec<_>, _>>()?;
                                let variant_doc = format!(
                                    "The archived counterpart of [`{}::{}`]",
                                    name, variant
                                );
                                Ok(quote! {
                                    #[doc = #variant_doc]
                                    #[allow(dead_code)]
                                    #variant(#(#fields,)*) = #discriminant
                                })
                            }
                            Fields::Unit => {
                                let variant_doc = format!(
                                    "The archived counterpart of [`{}::{}`]",
                                    name, variant
                                );
                                Ok(quote! {
                                    #[doc = #variant_doc]
                                    #[allow(dead_code)]
                                    #variant = #discriminant
                                })
                            }
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let archived_doc = archived_doc(name);
                let archive_attrs = &printing.archive_attrs;

                Some(quote! {
                    // SAFETY: As long as the `Archive` impl holds, the archived type is guaranteed to be `Portable`.
                    unsafe impl #impl_generics #rkyv_path::Portable for #archived_name #ty_generics #archive_where {}

                    #[automatically_derived]
                    #[doc = #archived_doc]
                    #(#archive_attrs)*
                    #repr
                    #vis enum #archived_name #generics #archive_where {
                        #(#archived_variants,)*
                    }
                })
            } else {
                None
            };

            let archived_variant_tags =
                data.variants.iter().enumerate().map(|(i, v)| {
                    let variant = &v.ident;
                    let discriminant = Literal::usize_unsuffixed(i);
                    quote! { #variant = #discriminant }
                });

            let archived_variant_structs = data.variants.iter().map(|v| -> Result<TokenStream, Error> {
                let variant = &v.ident;
                let archived_variant_name = Ident::new(&format!("ArchivedVariant{}", strip_raw(variant)), v.span());
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| -> Result<TokenStream, Error> {
                            let name = &f.ident;
                            let archived = archived(rkyv_path, f)?;
                            Ok(quote! { #name: #archived })
                        }).collect::<Result<Vec<_>, _>>()?;
                        Ok(quote! {
                            #[repr(C)]
                            struct #archived_variant_name #generics #archive_where {
                                __tag: ArchivedTag,
                                #(#fields,)*
                                __phantom: PhantomData<#name #ty_generics>,
                            }
                        })
                    }
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| -> Result<TokenStream, Error> {
                            let archived = archived(rkyv_path, f)?;
                            Ok(quote! { #archived })
                        }).collect::<Result<Vec<_>, _>>()?;
                        Ok(quote! {
                            #[repr(C)]
                            struct #archived_variant_name #generics (ArchivedTag, #(#fields,)* PhantomData<#name #ty_generics>) #archive_where;
                        })
                    }
                    Fields::Unit => Ok(quote! {})
                }
            }).collect::<Result<Vec<_>, _>>()?;

            let mut partial_eq_impl = None;
            let mut partial_ord_impl = None;
            if let Some(ref compares) = attributes.compares {
                for compare in compares {
                    if compare.is_ident("PartialEq") {
                        let mut partial_eq_where = archive_where.clone();
                        for variant in data.variants.iter() {
                            match variant.fields {
                                Fields::Named(ref fields) => {
                                    for field in fields
                                        .named
                                        .iter()
                                        .filter(is_not_omitted)
                                    {
                                        let ty = &field.ty;
                                        let archived =
                                            archived(rkyv_path, field)?;
                                        partial_eq_where.predicates.push(
                                            parse_quote! { #archived: PartialEq<#ty> },
                                        );
                                    }
                                }
                                Fields::Unnamed(ref fields) => {
                                    for field in fields
                                        .unnamed
                                        .iter()
                                        .filter(is_not_omitted)
                                    {
                                        let ty = &field.ty;
                                        let archived =
                                            archived(rkyv_path, field)?;
                                        partial_eq_where.predicates.push(
                                            parse_quote! { #archived: PartialEq<#ty> },
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
                                            Ident::new(&format!("self_{}", strip_raw(ident)), ident.span())
                                        })
                                    }).collect::<Vec<_>>();
                                    let other_bindings = fields.named.iter().map(|f| {
                                        f.ident.as_ref().map(|ident| {
                                            Ident::new(&format!("other_{}", strip_raw(ident)), ident.span())
                                        })
                                    }).collect::<Vec<_>>();
                                    quote! {
                                        #name::#variant { #(#field_names: #self_bindings,)* } => match other {
                                            #archived_name::#variant { #(#field_names: #other_bindings,)* } => true #(&& #other_bindings.eq(#self_bindings))*,
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
                                            #archived_name::#variant(#(#other_bindings,)*) => true #(&& #other_bindings.eq(#self_bindings))*,
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
                                }
                            }
                        });

                        partial_eq_impl = Some(quote! {
                            impl #impl_generics PartialEq<#archived_type> for #name #ty_generics #partial_eq_where {
                                #[inline]
                                fn eq(&self, other: &#archived_type) -> bool {
                                    match self {
                                        #(#variant_impls,)*
                                    }
                                }
                            }

                            impl #impl_generics PartialEq<#name #ty_generics> for #archived_type #partial_eq_where {
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
                                    for field in fields
                                        .named
                                        .iter()
                                        .filter(is_not_omitted)
                                    {
                                        let ty = &field.ty;
                                        let archived =
                                            archived(rkyv_path, field)?;
                                        partial_ord_where.predicates.push(
                                            parse_quote! { #archived: PartialOrd<#ty> },
                                        );
                                    }
                                }
                                Fields::Unnamed(ref fields) => {
                                    for field in fields
                                        .unnamed
                                        .iter()
                                        .filter(is_not_omitted)
                                    {
                                        let ty = &field.ty;
                                        let archived =
                                            archived(rkyv_path, field)?;
                                        partial_ord_where.predicates.push(
                                            parse_quote! { #archived: PartialOrd<#ty> },
                                        );
                                    }
                                }
                                Fields::Unit => (),
                            }
                        }

                        let self_disc =
                            data.variants.iter().enumerate().map(|(i, v)| {
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
                        let other_disc =
                            data.variants.iter().enumerate().map(|(i, v)| {
                                let variant = &v.ident;
                                match v.fields {
                                    Fields::Named(_) => quote! {
                                        #archived_name::#variant { .. } => #i
                                    },
                                    Fields::Unnamed(_) => quote! {
                                        #archived_name::#variant ( .. ) => #i
                                    },
                                    Fields::Unit => quote! {
                                        #archived_name::#variant => #i
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
                                            Ident::new(&format!("self_{}", strip_raw(ident)), ident.span())
                                        })
                                    }).collect::<Vec<_>>();
                                    let other_bindings = fields.named.iter().map(|f| {
                                        f.ident.as_ref().map(|ident| {
                                            Ident::new(&format!("other_{}", strip_raw(ident)), ident.span())
                                        })
                                    }).collect::<Vec<_>>();
                                    quote! {
                                        #name::#variant { #(#field_names: #self_bindings,)* } => match other {
                                            #archived_name::#variant { #(#field_names: #other_bindings,)* } => {
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
                                            #archived_name::#variant(#(#other_bindings,)*) => {
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
                                        #archived_name::#variant => Some(::core::cmp::Ordering::Equal),
                                        #[allow(unreachable_patterns)]
                                        _ => unsafe { ::core::hint::unreachable_unchecked() },
                                    }
                                }
                            }
                        });

                        partial_ord_impl = Some(quote! {
                            impl #impl_generics PartialOrd<#archived_type> for #name #ty_generics #partial_ord_where {
                                #[inline]
                                fn partial_cmp(&self, other: &#archived_type) -> Option<::core::cmp::Ordering> {
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

                            impl #impl_generics PartialOrd<#name #ty_generics> for #archived_type #partial_ord_where {
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
                        return Err(Error::new_spanned(
                            compare,
                            "unrecognized compare argument, supported \
                             compares are PartialEq (PartialOrd is not \
                             supported for enums)",
                        ));
                    }
                }
            }

            let resolver_doc = resolver_doc(name);

            (
                quote! {
                    #archived_def

                    #[automatically_derived]
                    #[doc = #resolver_doc]
                    #vis enum #resolver_name #generics #archive_where {
                        #(#resolver_variants,)*
                    }
                },
                quote! {
                    #repr
                    enum ArchivedTag {
                        #(#archived_variant_tags,)*
                    }

                    #(#archived_variant_structs)*

                    impl #impl_generics Archive for #name #ty_generics #archive_where {
                        type Archived = #archived_type;
                        type Resolver = #resolver_name #ty_generics;

                        // Some resolvers will be (), this allow is to prevent clippy from complaining
                        #[allow(clippy::unit_arg)]
                        #[inline]
                        unsafe fn resolve(&self, pos: usize, resolver: <Self as Archive>::Resolver, out: *mut <Self as Archive>::Archived) {
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

    let rkyv_path = &printing.rkyv_path;

    Ok(quote! {
        #archive_types

        #[automatically_derived]
        const _: () = {
            use core::marker::PhantomData;
            use #rkyv_path::{out_field, Archive, Archived};

            #archive_impls
        };
    })
}
