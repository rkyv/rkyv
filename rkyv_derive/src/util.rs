use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, Error, Field, Fields, Index, Member, Meta, Path, Type,
    WherePredicate,
};

pub fn strip_raw(ident: &Ident) -> String {
    let as_string = ident.to_string();
    as_string
        .strip_prefix("r#")
        .map(ToString::to_string)
        .unwrap_or(as_string)
}

pub fn is_not_omitted(f: &&Field) -> bool {
    f.attrs.iter().all(|attr| {
        if let Meta::Path(path) = &attr.meta {
            !path.is_ident("omit_bounds")
        } else {
            true
        }
    })
}

pub fn members(fields: &Fields) -> impl Iterator<Item = (Member, &Field)> {
    fields.iter().enumerate().map(|(i, field)| {
        let member = if let Some(ident) = &field.ident {
            Member::Named(ident.clone())
        } else {
            Member::Unnamed(Index::from(i))
        };
        (member, field)
    })
}

pub fn map_with_or_else<T>(
    field: &Field,
    f: impl FnOnce(Type) -> T,
    d: impl FnOnce() -> T,
) -> Result<T, Error> {
    let with_attr = field
        .attrs
        .iter()
        .find(|attr| attr.meta.path().is_ident("with"));
    if let Some(with) = with_attr {
        Ok(f(with.parse_args::<Type>()?))
    } else {
        Ok(d())
    }
}

pub fn archive_bound(
    rkyv_path: &Path,
    field: &Field,
) -> Result<WherePredicate, Error> {
    let ty = &field.ty;

    map_with_or_else(
        field,
        |with_ty| {
            parse_quote! {
                #with_ty: #rkyv_path::with::ArchiveWith<#ty>
            }
        },
        || {
            parse_quote! {
                #ty: #rkyv_path::Archive
            }
        },
    )
}

pub fn serialize_bound(
    rkyv_path: &Path,
    field: &Field,
) -> Result<WherePredicate, Error> {
    let ty = &field.ty;

    map_with_or_else(
        field,
        |with_ty| {
            parse_quote! {
                #with_ty: #rkyv_path::with::SerializeWith<#ty, __S>
            }
        },
        || {
            parse_quote! {
                #ty: #rkyv_path::Serialize<__S>
            }
        },
    )
}

pub fn deserialize_bound(
    rkyv_path: &Path,
    field: &Field,
) -> Result<WherePredicate, Error> {
    let ty = &field.ty;

    let archived = archived(rkyv_path, field)?;

    map_with_or_else(
        field,
        |with_ty| {
            parse_quote! {
                #with_ty: #rkyv_path::with::DeserializeWith<#archived, #ty, __D>
            }
        },
        || {
            parse_quote! {
                #archived: #rkyv_path::Deserialize<#ty, __D>
            }
        },
    )
}

fn archive_item(
    rkyv_path: &Path,
    field: &Field,
    name: &str,
    with_name: &str,
) -> Result<TokenStream, Error> {
    let ty = &field.ty;

    map_with_or_else(
        field,
        |with_ty| {
            let ident = Ident::new(with_name, Span::call_site());
            quote! {
                <#with_ty as #rkyv_path::with::ArchiveWith<#ty>>::#ident
            }
        },
        || {
            let ident = Ident::new(name, Span::call_site());
            quote! {
                <#ty as #rkyv_path::Archive>::#ident
            }
        },
    )
}

pub fn archived(rkyv_path: &Path, field: &Field) -> Result<TokenStream, Error> {
    archive_item(rkyv_path, field, "Archived", "Archived")
}

pub fn resolver(rkyv_path: &Path, field: &Field) -> Result<TokenStream, Error> {
    archive_item(rkyv_path, field, "Resolver", "Resolver")
}

pub fn resolve(rkyv_path: &Path, field: &Field) -> Result<TokenStream, Error> {
    archive_item(rkyv_path, field, "resolve", "resolve_with")
}

pub fn archived_doc(name: &Ident) -> String {
    format!("An archived [`{}`]", name)
}

pub fn resolver_doc(name: &Ident) -> String {
    format!("The resolver for an archived [`{}`]", name)
}

pub fn serialize(
    rkyv_path: &Path,
    field: &Field,
) -> Result<TokenStream, Error> {
    let ty = &field.ty;

    map_with_or_else(
        field,
        |with_ty| {
            quote! {
                <#with_ty as #rkyv_path::with::SerializeWith<#ty, __S>>::serialize_with
            }
        },
        || {
            quote! {
                <#ty as #rkyv_path::Serialize<__S>>::serialize
            }
        },
    )
}

pub fn deserialize(
    rkyv_path: &Path,
    field: &Field,
) -> Result<TokenStream, Error> {
    let ty = &field.ty;

    let archived = archived(rkyv_path, field)?;

    map_with_or_else(
        field,
        |with_ty| {
            quote! {
                <#with_ty as #rkyv_path::with::DeserializeWith<#archived, #ty, __D>>::deserialize_with
            }
        },
        || {
            quote! {
                <#archived as #rkyv_path::Deserialize<#ty, __D>>::deserialize
            }
        },
    )
}
