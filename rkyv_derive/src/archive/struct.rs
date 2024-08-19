use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, Error, Field, Fields, Generics,
};

use crate::{
    archive::{
        archive_field_metas, archived_doc, printing::Printing, resolver_doc,
    },
    attributes::Attributes,
    util::{archived, is_not_omitted, resolve, resolver},
};

pub fn impl_struct(
    printing: &Printing,
    generics: &Generics,
    attributes: &Attributes,
    fields: &Fields,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        name,
        archived_type,
        resolver_name,
        ..
    } = &printing;

    let mut result = TokenStream::new();

    if attributes.as_type.is_none() {
        result.extend(generate_archived_type(
            printing, generics, attributes, fields,
        )?);
    }

    result.extend(generate_resolver_type(printing, generics, fields)?);

    let mut resolve_statements = TokenStream::new();
    for (field, member) in fields.iter().zip(fields.members()) {
        let resolves = resolve(rkyv_path, field)?;
        resolve_statements.extend(quote! {
            let field_ptr = unsafe {
                ::core::ptr::addr_of_mut!((*out.ptr()).#member)
            };
            let field_out = unsafe {
                #rkyv_path::Place::from_field_unchecked(out, field_ptr)
            };
            #resolves(&self.#member, resolver.#member, field_out);
        });
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    result.extend(quote! {
        impl #impl_generics #rkyv_path::Archive for #name #ty_generics
        #where_clause
        {
            type Archived = #archived_type;
            type Resolver = #resolver_name #ty_generics;

            // Some resolvers will be (), this allow is to prevent clippy
            // from complaining.
            #[allow(clippy::unit_arg)]
            fn resolve(
                &self,
                resolver: Self::Resolver,
                out: #rkyv_path::Place<Self::Archived>,
            ) {
                #resolve_statements
            }
        }
    });

    for compare in attributes.compares.iter().flat_map(Punctuated::iter) {
        if compare.is_ident("PartialEq") {
            result
                .extend(generate_partial_eq_impl(printing, generics, fields)?);
        } else if compare.is_ident("PartialOrd") {
            result
                .extend(generate_partial_ord_impl(printing, generics, fields)?);
        } else {
            return Err(Error::new_spanned(
                compare,
                "unrecognized compare argument, supported compares are \
                 PartialEq and PartialOrd",
            ));
        }
    }

    Ok(result)
}

fn generate_archived_type(
    printing: &Printing,
    generics: &Generics,
    attributes: &Attributes,
    fields: &Fields,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        vis,
        name,
        archived_name,
        archived_metas,
        ..
    } = printing;

    let mut archived_fields = TokenStream::new();
    for field in fields {
        let Field {
            vis,
            ident,
            colon_token,
            ..
        } = field;
        let metas = archive_field_metas(attributes, field);
        let ty = archived(rkyv_path, field)?;

        archived_fields.extend(quote! {
            #(#[#metas])*
            #vis #ident #colon_token #ty,
        });
    }

    let where_clause = &generics.where_clause;
    let body = match fields {
        Fields::Named(_) => quote! { #where_clause { #archived_fields } },
        Fields::Unnamed(_) => quote! { (#archived_fields) #where_clause; },
        Fields::Unit => quote! { #where_clause; },
    };

    let doc_string = archived_doc(name);
    Ok(quote! {
        #[automatically_derived]
        #[doc = #doc_string]
        #(#[#archived_metas])*
        #[repr(C)]
        #vis struct #archived_name #generics #body
    })
}

fn generate_resolver_type(
    printing: &Printing,
    generics: &Generics,
    fields: &Fields,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        vis,
        name,
        resolver_name,
        ..
    } = printing;

    let mut resolver_fields = TokenStream::new();
    for field in fields.iter() {
        let Field {
            ident, colon_token, ..
        } = field;
        let ty = resolver(rkyv_path, field)?;

        resolver_fields.extend(quote! { #ident #colon_token #ty, });
    }

    let where_clause = &generics.where_clause;
    let body = match fields {
        Fields::Named(_) => quote! { #where_clause { #resolver_fields } },
        Fields::Unnamed(_) => quote! { (#resolver_fields) #where_clause; },
        Fields::Unit => quote! { #where_clause; },
    };

    let doc_string = resolver_doc(name);
    Ok(quote! {
        #[automatically_derived]
        #[doc = #doc_string]
        #vis struct #resolver_name #generics #body
    })
}

fn generate_partial_eq_impl(
    printing: &Printing,
    generics: &Generics,
    fields: &Fields,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        name,
        archived_type,
        ..
    } = printing;

    let mut where_clause = generics.where_clause.clone().unwrap();
    for field in fields.iter().filter(is_not_omitted) {
        let ty = &field.ty;
        let archived_ty = archived(rkyv_path, field)?;
        where_clause
            .predicates
            .push(parse_quote! { #archived_ty: PartialEq<#ty> });
    }

    let members = fields.members();
    let (impl_generics, ty_generics, _) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics PartialEq<#archived_type> for #name #ty_generics
        #where_clause
        {
            fn eq(&self, other: &#archived_type) -> bool {
                true #(&& other.#members.eq(&self.#members))*
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
    generics: &Generics,
    fields: &Fields,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        name,
        archived_type,
        ..
    } = printing;

    let mut where_clause = generics.where_clause.as_ref().unwrap().clone();

    for field in fields.iter().filter(is_not_omitted) {
        let ty = &field.ty;
        let archived_ty = archived(rkyv_path, field)?;
        where_clause
            .predicates
            .push(parse_quote! { #archived_ty: PartialOrd<#ty> });
    }

    let members = fields.members();
    let (impl_generics, ty_generics, _) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics PartialOrd<#archived_type>
            for #name #ty_generics
        #where_clause
        {
            fn partial_cmp(
                &self,
                other: &#archived_type,
            ) -> Option<::core::cmp::Ordering> {
                #(
                    match other.#members.partial_cmp(&self.#members) {
                        Some(::core::cmp::Ordering::Equal) => (),
                        x => return x.map(::core::cmp::Ordering::reverse),
                    }
                )*
                Some(::core::cmp::Ordering::Equal)
            }
        }

        impl #impl_generics PartialOrd<#name #ty_generics> for #archived_type
        #where_clause
        {
            fn partial_cmp(
                &self,
                other: &#name #ty_generics,
            ) -> Option<::core::cmp::Ordering> {
                other.partial_cmp(self).map(::core::cmp::Ordering::reverse)
            }
        }
    })
}
