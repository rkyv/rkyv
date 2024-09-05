use core::iter::FlatMap;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream}, parse_quote, punctuated::Iter, token, Data, DataEnum, DataStruct, DataUnion, Error, Field, Ident, MacroDelimiter, Meta, MetaList, Path, Token, Type, Variant, WherePredicate
};

pub fn try_set_attribute<T: ToTokens>(
    attribute: &mut Option<T>,
    value: T,
    name: &'static str,
) -> Result<(), Error> {
    if attribute.is_none() {
        *attribute = Some(value);
        Ok(())
    } else {
        Err(Error::new_spanned(
            value,
            format!("{} already specified", name),
        ))
    }
}

pub fn strip_raw(ident: &Ident) -> String {
    let as_string = ident.to_string();
    as_string
        .strip_prefix("r#")
        .map(ToString::to_string)
        .unwrap_or(as_string)
}

type VariantFieldsFn = fn(&Variant) -> Iter<'_, Field>;

fn variant_fields(variant: &Variant) -> Iter<'_, Field> {
    variant.fields.iter()
}

pub enum FieldsIter<'a> {
    Struct(Iter<'a, Field>),
    Enum(FlatMap<Iter<'a, Variant>, Iter<'a, Field>, VariantFieldsFn>),
}

impl<'a> Iterator for FieldsIter<'a> {
    type Item = &'a Field;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Struct(iter) => iter.next(),
            Self::Enum(iter) => iter.next(),
        }
    }
}

pub fn iter_fields(data: &Data) -> FieldsIter<'_> {
    match data {
        Data::Struct(DataStruct { fields, .. }) => {
            FieldsIter::Struct(fields.iter())
        }
        Data::Enum(DataEnum { variants, .. }) => {
            FieldsIter::Enum(variants.iter().flat_map(variant_fields))
        }
        Data::Union(DataUnion { fields, .. }) => {
            FieldsIter::Struct(fields.named.iter())
        }
    }
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

enum TypeOrMeta {
    Type(Type),
    List(MetaList),
    NamedType(MetaNamedType),
}

struct MetaNamedType {
    path: Path,
    value: Type,
}

impl TypeOrMeta {
    fn parse_items<T: Default>(
        input: ParseStream,
        mut logic: impl FnMut(&mut T, Self) -> Result<(), Error>,
    ) -> Result<T, Error> {
        let mut result = T::default();

        loop {
            logic(&mut result, input.parse()?)?;

            if input.is_empty() {
                return Ok(result);
            }

            input.parse::<Token![,]>()?;

            if input.is_empty() {
                return Ok(result);
            }
        }
    }
}

impl Parse for TypeOrMeta {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        match input.parse()? {
            Type::Path(ty_path) if input.peek(token::Paren) => {
                let (delimiter, tokens) = input.step(|cursor| {
                    if let Some((TokenTree::Group(g), rest)) =
                        cursor.token_tree()
                    {
                        let span = g.delim_span();

                        let delimiter = match g.delimiter() {
                            Delimiter::Parenthesis => {
                                MacroDelimiter::Paren(token::Paren(span))
                            }
                            _ => {
                                return Err(cursor.error("expected parentheses"))
                            }
                        };

                        Ok(((delimiter, g.stream()), rest))
                    } else {
                        Err(cursor.error("expected delimiter"))
                    }
                })?;

                return Ok(Self::List(MetaList {
                    path: ty_path.path,
                    delimiter,
                    tokens,
                }));
            }
            Type::Path(ty_path) if input.peek(Token![=]) => {
                input.parse::<Token![=]>()?;

                Ok(Self::NamedType(MetaNamedType {
                    path: ty_path.path,
                    value: input.parse()?,
                }))
            }
            ty => Ok(Self::Type(ty)),
        }
    }
}

impl ToTokens for TypeOrMeta {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            TypeOrMeta::Type(ty) => ty.to_tokens(tokens),
            TypeOrMeta::List(meta) => meta.path.to_tokens(tokens),
            TypeOrMeta::NamedType(meta) => meta.path.to_tokens(tokens),
        }
    }
}

#[derive(Default)]
struct With {
    ty: Option<Type>,
    remote: Option<Remote>,
}

impl With {
    fn from_field(field: &Field) -> Result<Self, Error> {
        let with_attr = field
            .attrs
            .iter()
            .find(|attr| attr.meta.path().is_ident("with"));

        if let Some(with) = with_attr {
            with.parse_args()
        } else {
            Ok(Self::default())
        }
    }

    fn parse_type_or_meta(
        &mut self,
        type_or_meta: TypeOrMeta,
    ) -> Result<(), Error> {
        match type_or_meta {
            TypeOrMeta::Type(ty) => {
                try_set_attribute(&mut self.ty, ty, "with-wrapper type")
            }
            TypeOrMeta::List(meta) if meta.path.is_ident("remote") => {
                let raw = meta.parse_args()?;
                let remote = Remote::from_raw(meta.path, raw);
                try_set_attribute(&mut self.remote, remote, "remote")
            }
            _ => Err(Error::new_spanned(
                type_or_meta,
                "unrecognized `with` argument",
            )),
        }
    }
}

impl Parse for With {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        TypeOrMeta::parse_items(input, Self::parse_type_or_meta)
    }
}

struct Remote {
    path: Path,
    field: Option<Type>,
    with: Option<Type>,
    getter: Option<Path>,
}

impl Remote {
    fn from_raw(path: Path, raw: RemoteRaw) -> Self {
        Self {
            path,
            field: raw.field,
            with: raw.with,
            getter: raw.getter,
        }
    }
}

impl ToTokens for Remote {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
    }
}

#[derive(Default)]
struct RemoteRaw {
    field: Option<Type>,
    with: Option<Type>,
    getter: Option<Path>,
}

impl RemoteRaw {
    fn parse_type_or_meta(
        &mut self,
        type_or_meta: TypeOrMeta,
    ) -> Result<(), Error> {
        match type_or_meta {
            TypeOrMeta::Type(ty) => {
                try_set_attribute(&mut self.field, ty, "remote field type")
            }
            TypeOrMeta::NamedType(meta) if meta.path.is_ident("with") => {
                try_set_attribute(&mut self.with, meta.value, "with")
            }
            TypeOrMeta::NamedType(meta) if meta.path.is_ident("getter") => {
                let Type::Path(ty_path) = meta.value else {
                    return Err(Error::new_spanned(
                        meta.value,
                        "expected path",
                    ));
                };

                try_set_attribute(&mut self.getter, ty_path.path, "getter")
            }
            _ => Err(Error::new_spanned(
                type_or_meta,
                "unrecognized `remote` argument",
            )),
        }
    }
}

impl Parse for RemoteRaw {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        TypeOrMeta::parse_items(input, Self::parse_type_or_meta)
    }
}

pub fn map_with_or_else<T>(
    field: &Field,
    f: impl FnOnce(Type) -> T,
    d: impl FnOnce() -> T,
) -> Result<T, Error> {
    Ok(With::from_field(field)?.ty.map_or_else(d, f))
}

pub fn map_with_remote_or_else<T>(
    field: &Field,
    f: impl FnOnce(Option<&Type>, &Type, Type) -> T,
    d: impl FnOnce() -> T,
) -> Result<T, Error> {
    let with = With::from_field(field)?;

    if let Some(remote) = with.remote {
        let field_ty = &field.ty;
        let remote_ty = remote.field.as_ref().unwrap_or(field_ty);

        if let Some(remote_with) = remote.with {
            let with_ty = with.ty.as_ref();

            return Ok(f(with_ty, remote_ty, remote_with));
        }
    }

    Ok(d())
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

pub fn archive_remote_bound(
    rkyv_path: &Path,
    field: &Field,
) -> Result<Option<WherePredicate>, Error> {
    let ty = &field.ty;

    map_with_remote_or_else(
        field,
        |with_ty, remote_ty, remote_with_ty| {
            Some(if let Some(with_ty) = with_ty {
                parse_quote! {
                    #remote_with_ty: #rkyv_path::with::ArchiveWith<
                        #remote_ty,
                        Archived = <#with_ty as #rkyv_path::with::ArchiveWith<#ty>>::Archived,
                        Resolver = <#with_ty as #rkyv_path::with::ArchiveWith<#ty>>::Resolver,
                    >
                }
            } else {
                parse_quote! {
                    #remote_with_ty: #rkyv_path::with::ArchiveWith<
                        #remote_ty,
                        Archived = <#ty as Archive>::Archived,
                        Resolver = <#ty as Archive>::Resolver,
                    >
                }
            })
        },
        || None,
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

pub fn serialize_remote_bound(
    rkyv_path: &Path,
    field: &Field,
) -> Result<Option<WherePredicate>, Error> {
    let ty = &field.ty;

    map_with_remote_or_else(
        field,
        |with_ty, remote_ty, remote_with_ty| {
            Some(if let Some(with_ty) = with_ty {
                parse_quote! {
                    #remote_with_ty: #rkyv_path::with::SerializeWith<
                        #remote_ty,
                        __S,
                        Archived = <#with_ty as #rkyv_path::with::ArchiveWith<#ty>>::Archived,
                        Resolver = <#with_ty as #rkyv_path::with::ArchiveWith<#ty>>::Resolver,
                    >
                }
            } else {
                parse_quote! {
                    #remote_with_ty: #rkyv_path::with::SerializeWith<
                        #remote_ty,
                        __S,
                        Archived = <#ty as Archive>::Archived,
                        Resolver = <#ty as Archive>::Resolver,
                    >
                }
            })
        },
        || None,
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

fn archive_remote_item(
    rkyv_path: &Path,
    field: &Field,
    name: &str,
    with_name: &str,
) -> Result<TokenStream, Error> {
    let ty = &field.ty;

    map_with_remote_or_else(
        field,
        |_with_ty, remote_ty, remote_with_ty| {
            let ident = Ident::new(with_name, Span::call_site());
            quote! {
                <
                    #remote_with_ty as #rkyv_path::with::ArchiveWith<#remote_ty>
                >::#ident
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

pub fn resolve_remote(
    rkyv_path: &Path,
    field: &Field,
) -> Result<TokenStream, Error> {
    archive_remote_item(rkyv_path, field, "resolve", "resolve_with")
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
                <
                    #with_ty as #rkyv_path::with::SerializeWith<#ty, __S>
                >::serialize_with
            }
        },
        || {
            quote! {
                <#ty as #rkyv_path::Serialize<__S>>::serialize
            }
        },
    )
}

pub fn serialize_remote(
    rkyv_path: &Path,
    field: &Field,
) -> Result<TokenStream, Error> {
    let ty = &field.ty;

    map_with_remote_or_else(
        field,
        |_with_ty, remote_ty, remote_with_ty| {
            quote! {
                <
                    #remote_with_ty as #rkyv_path::with::SerializeWith<#remote_ty, __S>
                >::serialize_with
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
                <
                    #with_ty as #rkyv_path::with::DeserializeWith<
                        #archived,
                        #ty,
                        __D,
                    >
                >::deserialize_with
            }
        },
        || {
            quote! {
                <#archived as #rkyv_path::Deserialize<#ty, __D>>::deserialize
            }
        },
    )
}

pub fn remote_field_access(field: &Field, member: &impl ToTokens) -> Result<TokenStream, Error> {
    let with = With::from_field(field)?;

    if let Some(remote) = with.remote {
        if let Some(getter) = remote.getter {
            return Ok(quote!(&#getter(field)));
        }
    }

    Ok(quote!(&field.#member))
}