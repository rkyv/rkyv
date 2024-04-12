use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{
    parse_quote, spanned::Spanned as _, Data, DataEnum, DeriveInput, Error,
    Fields, Ident,
};

use crate::{
    archive::{
        archived_doc, enum_field_doc, enum_resolver_field_doc,
        field_archive_attrs, printing::Printing, resolver_doc,
        resolver_variant_doc, variant_doc,
    },
    attributes::Attributes,
    util::{
        archive_bound, archived, is_not_omitted, members_starting_at, resolve,
        resolver, strip_raw,
    },
};

pub fn impl_enum(
    input: &mut DeriveInput,
    attributes: &Attributes,
    printing: &Printing,
) -> Result<(TokenStream, TokenStream), Error> {
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => unreachable!(),
    };

    if data.variants.len() > 256 {
        return Err(Error::new_spanned(
            &input.ident,
            "enums with more than 256 variants cannot derive Archive",
        ));
    }

    let rkyv_path = &printing.rkyv_path;

    let where_clause = input.generics.make_where_clause();

    for field in data
        .variants
        .iter()
        .flat_map(|v| v.fields.iter())
        .filter(is_not_omitted)
    {
        where_clause
            .predicates
            .push(archive_bound(rkyv_path, field)?);
    }

    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let archived_def = attributes
        .archive_as
        .is_none()
        .then(|| generate_archived_def(input, printing, data))
        .transpose()?;

    let resolver_def = generate_resolver_def(input, printing, data)?;
    let resolve_arms = generate_resolve_arms(input, printing, data)?;

    let archived_variant_tags =
        data.variants.iter().enumerate().map(|(i, v)| {
            let variant = &v.ident;
            let discriminant = Literal::usize_unsuffixed(i);
            quote! { #variant = #discriminant }
        });

    let archived_variant_structs =
        generate_variant_structs(input, printing, data)?;

    let mut partial_eq_impl = None;
    let mut partial_ord_impl = None;
    if let Some(ref compares) = attributes.compares {
        for compare in compares {
            if compare.is_ident("PartialEq") {
                partial_eq_impl =
                    Some(generate_partial_eq_impl(input, data, printing)?);
            } else if compare.is_ident("PartialOrd") {
                partial_ord_impl =
                    Some(generate_partial_ord_impl(input, data, printing)?);
            } else {
                return Err(Error::new_spanned(
                    compare,
                    "unrecognized compare argument, supported compares are \
                     PartialEq (PartialOrd is not supported for enums)",
                ));
            }
        }
    }

    let name = &input.ident;
    let archived_type = &printing.archived_type;
    let resolver_name = &printing.resolver_name;

    Ok((
        quote! {
            #archived_def
            #resolver_def
        },
        quote! {
            #[repr(u8)]
            enum ArchivedTag {
                #(#archived_variant_tags,)*
            }

            #(#archived_variant_structs)*

            impl #impl_generics Archive for #name #ty_generics #where_clause {
                type Archived = #archived_type;
                type Resolver = #resolver_name #ty_generics;

                // Some resolvers will be (), this allow is to prevent clippy
                // from complaining
                #[allow(clippy::unit_arg)]
                #[inline]
                unsafe fn resolve(
                    &self,
                    pos: usize,
                    resolver: <Self as Archive>::Resolver,
                    out: *mut <Self as Archive>::Archived,
                ) {
                    match resolver {
                        #(#resolve_arms,)*
                    }
                }
            }

            #partial_eq_impl
            #partial_ord_impl
        },
    ))
}

fn generate_archived_def(
    input: &DeriveInput,
    printing: &Printing,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let rkyv_path = &printing.rkyv_path;

    let archived_variants = data
        .variants
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let variant = &v.ident;
            let discriminant = Literal::usize_unsuffixed(i);

            let variant_doc = variant_doc(name, variant);

            match v.fields {
                Fields::Named(ref fields) => {
                    let fields = fields
                        .named
                        .iter()
                        .map(|f| {
                            let field_name = f.ident.as_ref();
                            let vis = &f.vis;
                            let field_doc = enum_field_doc(
                                name,
                                variant,
                                field_name.unwrap(),
                            );
                            let archive_attrs = field_archive_attrs(f);
                            let archived = archived(rkyv_path, f)?;
                            Ok(quote! {
                                #[doc = #field_doc]
                                #(#[#archive_attrs])*
                                #vis #field_name: #archived
                            })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;

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
                        .map(|(i, f)| {
                            let vis = &f.vis;
                            let field_doc = enum_field_doc(name, variant, &i);
                            let archive_attrs = field_archive_attrs(f);
                            let archived = archived(rkyv_path, f)?;
                            Ok(quote! {
                                #[doc = #field_doc]
                                #(#[#archive_attrs])*
                                #vis #archived
                            })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;

                    Ok(quote! {
                        #[doc = #variant_doc]
                        #[allow(dead_code)]
                        #variant(#(#fields,)*) = #discriminant
                    })
                }
                Fields::Unit => Ok(quote! {
                    #[doc = #variant_doc]
                    #[allow(dead_code)]
                    #variant = #discriminant
                }),
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let archived_doc = archived_doc(&input.ident);
    let archive_attrs = &printing.archive_attrs;

    let vis = &input.vis;
    let archived_name = &printing.archived_name;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        #[doc = #archived_doc]
        #(#archive_attrs)*
        #[repr(u8)]
        #vis enum #archived_name #generics #where_clause {
            #(#archived_variants,)*
        }

        // SAFETY: As long as the `Archive` impl holds, the archived type is
        // guaranteed to be `Portable`.
        unsafe impl #impl_generics #rkyv_path::Portable
            for #archived_name #ty_generics
        #where_clause
        {}
    })
}

fn generate_resolver_def(
    input: &DeriveInput,
    printing: &Printing,
    data: &DataEnum,
) -> Result<TokenStream, Error> {
    let rkyv_path = &printing.rkyv_path;
    let name = &input.ident;

    let resolver_variants = data
        .variants
        .iter()
        .map(|v| {
            let variant = &v.ident;
            let variant_doc = resolver_variant_doc(name, variant);

            match v.fields {
                Fields::Named(ref fields) => {
                    let fields = fields
                        .named
                        .iter()
                        .map(|f| {
                            let field_name = f.ident.as_ref().unwrap();
                            let resolver = resolver(rkyv_path, f)?;
                            let field_doc = enum_resolver_field_doc(
                                name, variant, field_name,
                            );
                            Ok(quote! {
                                #[doc = #field_doc]
                                #field_name: #resolver
                            })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;

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
                        .map(|(i, f)| {
                            let resolver = resolver(rkyv_path, f)?;
                            let field_doc =
                                enum_resolver_field_doc(name, variant, &i);
                            Ok(quote! {
                                #[doc = #field_doc]
                                #resolver
                            })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;

                    Ok(quote! {
                        #[doc = #variant_doc]
                        #[allow(dead_code)]
                        #variant(#(#fields,)*)
                    })
                }
                Fields::Unit => Ok(quote! {
                    #[doc = #variant_doc]
                    #[allow(dead_code)]
                    #variant
                }),
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let resolver_doc = resolver_doc(name);

    let vis = &input.vis;
    let resolver_name = &printing.resolver_name;
    let generics = &input.generics;
    let where_clause = generics.where_clause.as_ref().unwrap();

    Ok(quote! {
        #[automatically_derived]
        #[doc = #resolver_doc]
        #vis enum #resolver_name #generics #where_clause {
            #(#resolver_variants,)*
        }
    })
}

fn generate_resolve_arms(
    input: &DeriveInput,
    printing: &Printing,
    data: &DataEnum,
) -> Result<Vec<TokenStream>, Error> {
    let rkyv_path = &printing.rkyv_path;
    let name = &input.ident;
    let resolver_name = &printing.resolver_name;
    let (_, ty_generics, _) = input.generics.split_for_impl();

    data.variants.iter().map(|v| {
        let variant = &v.ident;
        let archived_variant_name = Ident::new(
            &format!("ArchivedVariant{}", strip_raw(variant)),
            v.span(),
        );

        let members = members_starting_at(&v.fields, 1)
            .map(|(m, _)| m)
            .collect::<Vec<_>>();

        let (self_bindings, resolver_bindings) = v.fields
            .iter()
            .enumerate()
            .map(|(i, field)| (
                Ident::new(&format!("self_{}", i), field.span()),
                Ident::new(&format!("resolver_{}", i), field.span()),
            ))
            .unzip::<_, _, Vec<_>, Vec<_>>();

        let resolves = v.fields
            .iter()
            .map(|f| resolve(rkyv_path, f))
            .collect::<Result<Vec<_>, Error>>()?;

        match v.fields {
            Fields::Named(_) => Ok(quote! {
                #resolver_name::#variant {
                    #(#members: #resolver_bindings,)*
                } => {
                    match self {
                        #name::#variant {
                            #(#members: #self_bindings,)*
                        } => {
                            let out = out
                                .cast::<#archived_variant_name #ty_generics>();
                            ::core::ptr::addr_of_mut!((*out).__tag)
                                .write(ArchivedTag::#variant);
                            #(
                                let (fp, fo) = out_field!(out.#members);
                                #resolves(
                                    #self_bindings,
                                    pos + fp,
                                    #resolver_bindings,
                                    fo,
                                );
                            )*
                        },
                        #[allow(unreachable_patterns)]
                        _ => ::core::hint::unreachable_unchecked(),
                    }
                }
            }),
            Fields::Unnamed(_) => Ok(quote! {
                #resolver_name::#variant( #(#resolver_bindings,)* ) => {
                    match self {
                        #name::#variant(#(#self_bindings,)*) => {
                            let out = out
                                .cast::<#archived_variant_name #ty_generics>();
                            ::core::ptr::addr_of_mut!((*out).0)
                                .write(ArchivedTag::#variant);
                            #(
                                let (fp, fo) = out_field!(out.#members);
                                #resolves(
                                    #self_bindings,
                                    pos + fp,
                                    #resolver_bindings,
                                    fo,
                                );
                            )*
                        },
                        #[allow(unreachable_patterns)]
                        _ => ::core::hint::unreachable_unchecked(),
                    }
                }
            }),
            Fields::Unit => Ok(quote! {
                #resolver_name::#variant => {
                    out.cast::<ArchivedTag>().write(ArchivedTag::#variant);
                }
            }),
        }
    }).collect()
}

fn generate_variant_structs(
    input: &DeriveInput,
    printing: &Printing,
    data: &DataEnum,
) -> Result<Vec<TokenStream>, Error> {
    let rkyv_path = &printing.rkyv_path;
    let name = &input.ident;
    let generics = &input.generics;
    let where_clause = generics.where_clause.as_ref().unwrap();
    let (_, ty_generics, _) = input.generics.split_for_impl();

    data.variants
        .iter()
        .map(|v| {
            let variant = &v.ident;
            let archived_variant_name = Ident::new(
                &format!("ArchivedVariant{}", strip_raw(variant)),
                v.span(),
            );

            match v.fields {
                Fields::Named(ref fields) => {
                    let fields = fields
                        .named
                        .iter()
                        .map(|f| {
                            let name = &f.ident;
                            let archived = archived(rkyv_path, f)?;
                            Ok(quote! { #name: #archived })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;
                    Ok(quote! {
                        #[repr(C)]
                        struct #archived_variant_name #generics #where_clause {
                            __tag: ArchivedTag,
                            #(#fields,)*
                            __phantom: PhantomData<#name #ty_generics>,
                        }
                    })
                }
                Fields::Unnamed(ref fields) => {
                    let fields = fields
                        .unnamed
                        .iter()
                        .map(|f| {
                            let archived = archived(rkyv_path, f)?;
                            Ok(quote! { #archived })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;
                    Ok(quote! {
                        #[repr(C)]
                        struct #archived_variant_name #generics (
                            ArchivedTag,
                            #(#fields,)*
                            PhantomData<#name #ty_generics>,
                        ) #where_clause;
                    })
                }
                Fields::Unit => Ok(quote! {}),
            }
        })
        .collect()
}

fn generate_partial_eq_impl(
    input: &DeriveInput,
    data: &DataEnum,
    printing: &Printing,
) -> Result<TokenStream, Error> {
    let mut partial_eq_where =
        input.generics.where_clause.as_ref().unwrap().clone();

    for field in data
        .variants
        .iter()
        .flat_map(|v| v.fields.iter())
        .filter(is_not_omitted)
    {
        let ty = &field.ty;
        let archived = archived(&printing.rkyv_path, field)?;
        partial_eq_where
            .predicates
            .push(parse_quote! { #archived: PartialEq<#ty> });
    }

    let archived_name = &printing.archived_name;
    let archived_type = &printing.archived_type;
    let name = &input.ident;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

    let variant_impls = data.variants.iter().map(|v| {
        let variant = &v.ident;

        let (self_fields, other_fields) = v
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    Ident::new(&format!("self_{}", i), f.span()),
                    Ident::new(&format!("other_{}", i), f.span()),
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
        #partial_eq_where
        {
            #[inline]
            fn eq(&self, other: &#archived_type) -> bool {
                match self {
                    #(#variant_impls,)*
                }
            }
        }

        impl #impl_generics PartialEq<#name #ty_generics> for #archived_type
        #partial_eq_where
        {
            #[inline]
            fn eq(&self, other: &#name #ty_generics) -> bool {
                other.eq(self)
            }
        }
    })
}

fn generate_partial_ord_impl(
    input: &DeriveInput,
    data: &DataEnum,
    printing: &Printing,
) -> Result<TokenStream, Error> {
    let mut partial_ord_where =
        input.generics.where_clause.as_ref().unwrap().clone();

    for field in data
        .variants
        .iter()
        .flat_map(|v| v.fields.iter())
        .filter(is_not_omitted)
    {
        let ty = &field.ty;
        let archived = archived(&printing.rkyv_path, field)?;
        partial_ord_where
            .predicates
            .push(parse_quote! { #archived: PartialOrd<#ty> });
    }

    let archived_name = &printing.archived_name;
    let archived_type = &printing.archived_type;
    let name = &input.ident;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

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

        let (self_fields, other_fields) = v
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    Ident::new(&format!("self_{}", i), f.span()),
                    Ident::new(&format!("other_{}", i), f.span()),
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
            Fields::Unnamed(_) => {
                quote! {
                    #name::#variant(#(#self_fields,)*) => match other {
                        #archived_name::#variant(#(#other_fields,)*) => {
                            #(
                                match #other_fields.partial_cmp(#self_fields) {
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
        #partial_ord_where
        {
            #[inline]
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
        #partial_ord_where
        {
            #[inline]
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
