use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, Data, DeriveInput, Error, Fields, FieldsNamed, FieldsUnnamed,
};

use crate::{
    archive::{field_archive_attrs, printing::Printing},
    attributes::Attributes,
    util::{
        archive_bound, archived, archived_doc, is_not_omitted, members,
        resolve, resolver, resolver_doc,
    },
};

pub fn impl_struct(
    input: &mut DeriveInput,
    attributes: &Attributes,
    printing: &Printing,
) -> Result<(TokenStream, TokenStream), Error> {
    let rkyv_path = &printing.rkyv_path;

    let where_clause = input.generics.make_where_clause();

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => unreachable!(),
    };
    for field in fields.iter().filter(is_not_omitted) {
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
        .then(|| -> Result<TokenStream, Error> {
            let def = generate_archived_def(input, printing, fields)?;
            let archived_name = &printing.archived_name;

            Ok(quote! {
                #def

                // SAFETY: As long as the `Archive` impl holds, the archived type is
                // guaranteed to be `Portable`.
                unsafe impl #impl_generics #rkyv_path::Portable
                    for #archived_name #ty_generics #where_clause {}
            })
        })
        .transpose()?;

    let resolver_def = generate_resolver_def(input, printing, fields)?;

    let resolve_statements = members(fields)
        .map(|(member, field)| -> Result<TokenStream, Error> {
            let resolve_fn = resolve(rkyv_path, field)?;
            Ok(quote! {
                let (fp, fo) = out_field!(out.#member);
                #resolve_fn(&self.#member, pos + fp, resolver.#member, fo);
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut partial_eq_impl = None;
    let mut partial_ord_impl = None;
    if let Some(ref compares) = attributes.compares {
        for compare in compares {
            if compare.is_ident("PartialEq") {
                partial_eq_impl =
                    Some(generate_partial_eq_impl(input, fields, printing)?);
            } else if compare.is_ident("PartialOrd") {
                partial_ord_impl =
                    Some(generate_partial_ord_impl(input, fields, printing)?);
            } else {
                return Err(Error::new_spanned(
                    compare,
                    "unrecognized compare argument, supported compares are \
                     PartialEq and PartialOrd",
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
            impl #impl_generics #rkyv_path::Archive for #name #ty_generics #where_clause {
                type Archived = #archived_type;
                type Resolver = #resolver_name #ty_generics;

                // Some resolvers will be (), this allow is to prevent clippy from complaining
                #[allow(clippy::unit_arg)]
                #[inline]
                unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
                    #(#resolve_statements)*
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
    fields: &Fields,
) -> Result<TokenStream, Error> {
    match fields {
        Fields::Named(fields) => {
            generate_archived_def_named(input, printing, fields)
        }
        Fields::Unnamed(fields) => {
            generate_archived_def_unnamed(input, printing, fields)
        }
        Fields::Unit => generate_archived_def_unit(input, printing),
    }
}

fn generate_archived_def_named(
    input: &DeriveInput,
    printing: &Printing,
    fields: &FieldsNamed,
) -> Result<TokenStream, Error> {
    let rkyv_path = &printing.rkyv_path;

    let archived_fields = fields
        .named
        .iter()
        .map(|field| -> Result<TokenStream, Error> {
            let field_ty = archived(rkyv_path, field)?;
            let vis = &field.vis;
            let archive_attrs = field_archive_attrs(field);

            let field_name = field.ident.as_ref().unwrap();
            let field_doc = format!(
                "The archived counterpart of [`{}::{}`]",
                &input.ident, field_name,
            );
            Ok(quote! {
                #[doc = #field_doc]
                #(#[#archive_attrs])*
                #vis #field_name: #field_ty
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let archived_doc = archived_doc(&input.ident);
    let archive_attrs = &printing.archive_attrs;
    let vis = &input.vis;
    let archived_name = &printing.archived_name;
    let generics = &input.generics;
    let where_clause = generics.where_clause.as_ref().unwrap();

    Ok(quote! {
        #[automatically_derived]
        #[doc = #archived_doc]
        #(#archive_attrs)*
        #[repr(C)]
        #vis struct #archived_name #generics #where_clause {
            #(#archived_fields,)*
        }
    })
}

fn generate_archived_def_unnamed(
    input: &DeriveInput,
    printing: &Printing,
    fields: &FieldsUnnamed,
) -> Result<TokenStream, Error> {
    let rkyv_path = &printing.rkyv_path;

    let archived_fields = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, field)| -> Result<TokenStream, Error> {
            let field_doc = format!(
                "The archived counterpart of [`{}::{}`]",
                &input.ident, i,
            );
            let archive_attrs = field_archive_attrs(field);
            let vis = &field.vis;
            let field_ty = archived(rkyv_path, field)?;

            Ok(quote! {
                #[doc = #field_doc]
                #(#[#archive_attrs])*
                #vis #field_ty
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let archived_doc = archived_doc(&input.ident);
    let archive_attrs = &printing.archive_attrs;
    let vis = &input.vis;
    let archived_name = &printing.archived_name;
    let generics = &input.generics;
    let where_clause = generics.where_clause.as_ref().unwrap();

    Ok(quote! {
        #[automatically_derived]
        #[doc = #archived_doc]
        #(#archive_attrs)*
        #[repr(C)]
        #vis struct #archived_name #generics(
            #(#archived_fields,)*
        ) #where_clause;
    })
}

fn generate_archived_def_unit(
    input: &DeriveInput,
    printing: &Printing,
) -> Result<TokenStream, Error> {
    let archived_doc = archived_doc(&input.ident);
    let archive_attrs = &printing.archive_attrs;
    let vis = &input.vis;
    let archived_name = &printing.archived_name;
    let generics = &input.generics;
    let where_clause = generics.where_clause.as_ref().unwrap();

    Ok(quote! {
        #[automatically_derived]
        #[doc = #archived_doc]
        #(#archive_attrs)*
        #[repr(C)]
        #vis struct #archived_name #generics #where_clause;
    })
}

fn generate_resolver_def(
    input: &DeriveInput,
    printing: &Printing,
    fields: &Fields,
) -> Result<TokenStream, Error> {
    match fields {
        Fields::Named(fields) => {
            generate_resolver_def_named(input, printing, fields)
        }
        Fields::Unnamed(fields) => {
            generate_resolver_def_unnamed(input, printing, fields)
        }
        Fields::Unit => generate_resolver_def_unit(input, printing),
    }
}

fn generate_resolver_def_named(
    input: &DeriveInput,
    printing: &Printing,
    fields: &FieldsNamed,
) -> Result<TokenStream, Error> {
    let rkyv_path = &printing.rkyv_path;
    let resolver_name = &printing.resolver_name;
    let vis = &input.vis;
    let generics = &input.generics;
    let where_clause = generics.where_clause.as_ref().unwrap();
    let resolver_doc = resolver_doc(&input.ident);

    let resolver_fields = fields
        .named
        .iter()
        .map(|field| -> Result<TokenStream, Error> {
            let field_name = &field.ident;
            let resolver_ty = resolver(rkyv_path, field)?;

            Ok(quote! { #field_name: #resolver_ty })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(quote! {
        #[automatically_derived]
        #[doc = #resolver_doc]
        #vis struct #resolver_name #generics #where_clause {
            #(#resolver_fields,)*
        }
    })
}

fn generate_resolver_def_unnamed(
    input: &DeriveInput,
    printing: &Printing,
    fields: &FieldsUnnamed,
) -> Result<TokenStream, Error> {
    let rkyv_path = &printing.rkyv_path;
    let resolver_name = &printing.resolver_name;
    let vis = &input.vis;
    let generics = &input.generics;
    let where_clause = generics.where_clause.as_ref().unwrap();
    let resolver_doc = resolver_doc(&input.ident);

    let resolver_fields = fields
        .unnamed
        .iter()
        .map(|field| -> Result<TokenStream, Error> {
            let resolver_ty = resolver(rkyv_path, field)?;
            Ok(quote! { #resolver_ty })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(quote! {
        #[automatically_derived]
        #[doc = #resolver_doc]
        #vis struct #resolver_name #generics (
            #(#resolver_fields,)*
        ) #where_clause;
    })
}

fn generate_resolver_def_unit(
    input: &DeriveInput,
    printing: &Printing,
) -> Result<TokenStream, Error> {
    let resolver_name = &printing.resolver_name;
    let vis = &input.vis;
    let generics = &input.generics;
    let where_clause = generics.where_clause.as_ref().unwrap();
    let resolver_doc = resolver_doc(&input.ident);

    Ok(quote! {
        #[automatically_derived]
        #[doc = #resolver_doc]
        #vis struct #resolver_name #generics #where_clause;
    })
}

fn generate_partial_eq_impl(
    input: &DeriveInput,
    fields: &Fields,
    printing: &Printing,
) -> Result<TokenStream, Error> {
    let mut partial_eq_where =
        input.generics.where_clause.as_ref().unwrap().clone();

    for field in fields.iter().filter(is_not_omitted) {
        let ty = &field.ty;
        let archived_ty = archived(&printing.rkyv_path, field)?;
        partial_eq_where
            .predicates
            .push(parse_quote! { #archived_ty: PartialEq<#ty> });
    }

    let members = members(fields).map(|(member, _)| member);

    let archived_type = &printing.archived_type;
    let name = &input.ident;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics PartialEq<#archived_type> for #name #ty_generics #partial_eq_where {
            #[inline]
            fn eq(&self, other: &#archived_type) -> bool {
                true #(&& other.#members.eq(&self.#members))*
            }
        }

        impl #impl_generics PartialEq<#name #ty_generics> for #archived_type #partial_eq_where {
            #[inline]
            fn eq(&self, other: &#name #ty_generics) -> bool {
                other.eq(self)
            }
        }
    })
}

fn generate_partial_ord_impl(
    input: &DeriveInput,
    fields: &Fields,
    printing: &Printing,
) -> Result<TokenStream, Error> {
    let mut partial_ord_where =
        input.generics.where_clause.as_ref().unwrap().clone();

    for field in fields.iter().filter(is_not_omitted) {
        let ty = &field.ty;
        let archived_ty = archived(&printing.rkyv_path, field)?;
        partial_ord_where
            .predicates
            .push(parse_quote! { #archived_ty: PartialOrd<#ty> });
    }

    let members = members(fields).map(|(member, _)| member);

    let archived_type = &printing.archived_type;
    let name = &input.ident;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics PartialOrd<#archived_type> for #name #ty_generics #partial_ord_where {
            #[inline]
            fn partial_cmp(&self, other: &#archived_type) -> Option<::core::cmp::Ordering> {
                #(
                    match other.#members.partial_cmp(&self.#members) {
                        Some(::core::cmp::Ordering::Equal) => (),
                        x => return x,
                    }
                )*
                Some(::core::cmp::Ordering::Equal)
            }
        }

        impl #impl_generics PartialOrd<#name #ty_generics> for #archived_type #partial_ord_where {
            #[inline]
            fn partial_cmp(&self, other: &#name #ty_generics) -> Option<::core::cmp::Ordering> {
                other.partial_cmp(self)
            }
        }
    })
}
