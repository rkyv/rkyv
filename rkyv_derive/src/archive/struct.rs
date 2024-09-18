use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, Error, Field, Fields, Generics,
};

use crate::{
    archive::{archived_doc, printing::Printing, resolver_doc},
    attributes::{Attributes, FieldAttributes},
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
    } = printing;

    let mut result = TokenStream::new();

    if attributes.as_type.is_none() {
        result.extend(generate_archived_type(
            printing, generics, attributes, fields,
        )?);
    }

    result.extend(generate_resolver_type(
        printing, generics, attributes, fields,
    )?);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let archive_impl = if let Some(ref remote) = attributes.remote {
        let resolve_statements = generate_resolve_statements(
            printing,
            attributes,
            fields,
            Ident::new("field", Span::call_site()),
        )?;

        quote! {
            impl #impl_generics #rkyv_path::with::ArchiveWith<#remote>
                for #name #ty_generics
            #where_clause
            {
                type Archived = #archived_type;
                type Resolver = #resolver_name #ty_generics;

                // Some resolvers will be (), this allow is to prevent clippy
                // from complaining.
                #[allow(clippy::unit_arg)]
                fn resolve_with(
                    field: &#remote,
                    resolver: Self::Resolver,
                    out: #rkyv_path::Place<Self::Archived>,
                ) {
                    #resolve_statements
                }
            }
        }
    } else {
        let resolve_statements = generate_resolve_statements(
            printing,
            attributes,
            fields,
            Ident::new("self", Span::call_site()),
        )?;

        let copy_optimization =
            generate_copy_optimization(printing, generics, attributes, fields)?;

        quote! {
            impl #impl_generics #rkyv_path::Archive for #name #ty_generics
            #where_clause
            {
                type Archived = #archived_type;
                type Resolver = #resolver_name #ty_generics;

                #copy_optimization

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
        }
    };

    result.extend(archive_impl);

    for compare in attributes.compares.iter().flat_map(Punctuated::iter) {
        if compare.is_ident("PartialEq") {
            result.extend(generate_partial_eq_impl(
                printing, generics, attributes, fields,
            )?);
        } else if compare.is_ident("PartialOrd") {
            result.extend(generate_partial_ord_impl(
                printing, generics, attributes, fields,
            )?);
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

fn generate_resolve_statements(
    printing: &Printing,
    attributes: &Attributes,
    fields: &Fields,
    this: Ident,
) -> Result<TokenStream, Error> {
    let rkyv_path = &printing.rkyv_path;
    let mut resolve_statements = TokenStream::new();
    for (field, member) in fields.iter().zip(fields.members()) {
        let field_attrs = FieldAttributes::parse(attributes, field)?;
        let resolves = field_attrs.resolve(rkyv_path, field);
        let access_field = field_attrs.access_field(&this, &member);
        resolve_statements.extend(quote! {
            let field_ptr = unsafe {
                ::core::ptr::addr_of_mut!((*out.ptr()).#member)
            };
            let field_out = unsafe {
                #rkyv_path::Place::from_field_unchecked(out, field_ptr)
            };
            #resolves(#access_field, resolver.#member, field_out);
        });
    }
    Ok(resolve_statements)
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

        let field_attrs = FieldAttributes::parse(attributes, field)?;
        let field_metas = field_attrs.metas();
        let ty = field_attrs.archived(rkyv_path, field);

        archived_fields.extend(quote! {
            #field_metas
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
    attributes: &Attributes,
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
        let field_attrs = FieldAttributes::parse(attributes, field)?;

        let ty = field_attrs.resolver(rkyv_path, field);

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
    attributes: &Attributes,
    fields: &Fields,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        name,
        archived_type,
        ..
    } = printing;

    let mut where_clause = generics.where_clause.clone().unwrap();
    for field in fields.iter() {
        let field_attrs = FieldAttributes::parse(attributes, field)?;
        if field_attrs.omit_bounds.is_none() {
            let ty = &field.ty;
            let archived_ty = field_attrs.archived(rkyv_path, field);
            where_clause
                .predicates
                .push(parse_quote! { #archived_ty: PartialEq<#ty> });
        }
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
    attributes: &Attributes,
    fields: &Fields,
) -> Result<TokenStream, Error> {
    let Printing {
        rkyv_path,
        name,
        archived_type,
        ..
    } = printing;

    let mut where_clause = generics.where_clause.as_ref().unwrap().clone();

    for field in fields.iter() {
        let field_attrs = FieldAttributes::parse(attributes, field)?;
        if field_attrs.omit_bounds.is_none() {
            let ty = &field.ty;
            let archived_ty = field_attrs.archived(rkyv_path, field);
            where_clause
                .predicates
                .push(parse_quote! { #archived_ty: PartialOrd<#ty> });
        }
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

fn generate_copy_optimization(
    printing: &Printing,
    generics: &Generics,
    attributes: &Attributes,
    fields: &Fields,
) -> Result<Option<TokenStream>, Error> {
    if !generics.params.is_empty() {
        return Ok(None);
    }

    for f in fields.iter() {
        if FieldAttributes::parse(attributes, f)?.with.is_some() {
            return Ok(None);
        }
    }

    let Printing {
        rkyv_path,
        name,
        archived_type,
        ..
    } = printing;

    let field_sizes = fields.iter().map(|f| {
        let ty = &f.ty;

        quote! {
            ::core::mem::size_of::<#ty>()
        }
    });
    let padding_check = quote! {
        0 #(+ #field_sizes)* == ::core::mem::size_of::<#name>()
    };

    let field_checks = fields.iter().zip(fields.members()).map(|(f, m)| {
        let ty = &f.ty;

        quote! {
            <#ty as #rkyv_path::Archive>::COPY_OPTIMIZATION.is_enabled()
            && ::core::mem::offset_of!(#name, #m)
                == ::core::mem::offset_of!(#archived_type, #m)
        }
    });

    Ok(Some(quote! {
        const COPY_OPTIMIZATION: #rkyv_path::traits::CopyOptimization<Self> =
            unsafe {
                #rkyv_path::traits::CopyOptimization::enable_if(
                    #padding_check #(&& #field_checks)*
                )
            };
    }))
}
