use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    meta::ParseNestedMeta, parenthesized, parse::Parse, parse_quote,
    punctuated::Punctuated, DeriveInput, Error, Field, Fields, Ident, Meta,
    Path, Token, Type, Variant, WherePredicate,
};

fn try_set_attribute<T: ToTokens>(
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
            format!("{name} already specified"),
        ))
    }
}

#[derive(Default)]
pub struct Attributes {
    pub as_type: Option<Type>,
    pub archived: Option<Ident>,
    pub resolver: Option<Ident>,
    pub remote: Option<Path>,
    pub metas: Vec<Meta>,
    pub compares: Option<Punctuated<Path, Token![,]>>,
    pub archive_bounds: Option<Punctuated<WherePredicate, Token![,]>>,
    pub serialize_bounds: Option<Punctuated<WherePredicate, Token![,]>>,
    pub deserialize_bounds: Option<Punctuated<WherePredicate, Token![,]>>,
    pub bytecheck: Option<TokenStream>,
    pub crate_path: Option<Path>,
}

impl Attributes {
    fn parse_meta(&mut self, meta: ParseNestedMeta<'_>) -> Result<(), Error> {
        if meta.path.is_ident("bytecheck") {
            let tokens = meta.input.step(|cursor| {
                if let Some((TokenTree::Group(group), rest)) =
                    cursor.token_tree()
                {
                    Ok((group.stream(), rest))
                } else {
                    Err(cursor.error("expected bytecheck attributes"))
                }
            })?;

            if cfg!(feature = "bytecheck") {
                try_set_attribute(&mut self.bytecheck, tokens, "bytecheck")?;
            }

            Ok(())
        } else if meta.path.is_ident("compare") {
            let traits;
            parenthesized!(traits in meta.input);
            let traits = traits.parse_terminated(Path::parse, Token![,])?;
            try_set_attribute(&mut self.compares, traits, "compare")
        } else if meta.path.is_ident("archive_bounds") {
            let bounds;
            parenthesized!(bounds in meta.input);
            let clauses =
                bounds.parse_terminated(WherePredicate::parse, Token![,])?;
            try_set_attribute(
                &mut self.archive_bounds,
                clauses,
                "archive_bounds",
            )
        } else if meta.path.is_ident("serialize_bounds") {
            let bounds;
            parenthesized!(bounds in meta.input);
            let clauses =
                bounds.parse_terminated(WherePredicate::parse, Token![,])?;
            try_set_attribute(
                &mut self.serialize_bounds,
                clauses,
                "serialize_bounds",
            )
        } else if meta.path.is_ident("deserialize_bounds") {
            let bounds;
            parenthesized!(bounds in meta.input);
            let clauses =
                bounds.parse_terminated(WherePredicate::parse, Token![,])?;
            try_set_attribute(
                &mut self.deserialize_bounds,
                clauses,
                "deserialize_bounds",
            )
        } else if meta.path.is_ident("archived") {
            try_set_attribute(
                &mut self.archived,
                meta.value()?.parse()?,
                "archived",
            )
        } else if meta.path.is_ident("resolver") {
            try_set_attribute(
                &mut self.resolver,
                meta.value()?.parse()?,
                "resolver",
            )
        } else if meta.path.is_ident("as") {
            meta.input.parse::<Token![=]>()?;
            try_set_attribute(
                &mut self.as_type,
                meta.input.parse::<Type>()?,
                "as",
            )
        } else if meta.path.is_ident("crate") {
            if meta.input.parse::<Token![=]>().is_ok() {
                let path = meta.input.parse::<Path>()?;
                try_set_attribute(&mut self.crate_path, path, "crate")
            } else if meta.input.is_empty() || meta.input.peek(Token![,]) {
                try_set_attribute(
                    &mut self.crate_path,
                    parse_quote! { crate },
                    "crate",
                )
            } else {
                Err(meta.error("expected `crate` or `crate = ...`"))
            }
        } else if meta.path.is_ident("derive") {
            let metas;
            parenthesized!(metas in meta.input);
            self.metas.extend(
                metas
                    .parse_terminated(Meta::parse, Token![,])?
                    .into_iter()
                    .map(|meta| parse_quote! { derive(#meta) }),
            );
            Ok(())
        } else if meta.path.is_ident("attr") {
            let metas;
            parenthesized!(metas in meta.input);
            self.metas
                .extend(metas.parse_terminated(Meta::parse, Token![,])?);
            Ok(())
        } else if meta.path.is_ident("remote") {
            try_set_attribute(
                &mut self.remote,
                meta.value()?.parse()?,
                "remote",
            )
        } else {
            Err(meta.error("unrecognized rkyv argument"))
        }
    }

    pub fn parse(input: &DeriveInput) -> Result<Self, Error> {
        let mut result = Self::default();

        for attr in input.attrs.iter() {
            if attr.path().is_ident("rkyv") {
                attr.parse_nested_meta(|meta| result.parse_meta(meta))?;
            }
        }

        if result.as_type.is_some() {
            if let Some(ref ident) = result.archived {
                return Err(Error::new_spanned(
                    ident,
                    "`archived = ...` may not be used with `as = ...` because \
                     no type is generated",
                ));
            }

            if let Some(first) = result.metas.first() {
                return Err(Error::new_spanned(
                    first,
                    "attributes may not be used with `as = ...`; place \
                     attributes on the archived type instead",
                ));
            }

            if let Some(bytecheck) = &result.bytecheck {
                return Err(Error::new_spanned(
                    bytecheck,
                    "cannot generate a `CheckBytes` impl because `as = ...` \
                     does not generate an archived type",
                ));
            }
        }

        Ok(result)
    }

    pub fn crate_path(&self) -> Path {
        self.crate_path
            .clone()
            .unwrap_or_else(|| parse_quote! { ::rkyv })
    }
}

#[derive(Default)]
pub struct FieldAttributes {
    pub attrs: Punctuated<Meta, Token![,]>,
    pub omit_bounds: Option<Path>,
    pub with: Option<Type>,
    pub getter: Option<Path>,
    pub niches: Vec<Niche>,
}

impl FieldAttributes {
    fn parse_meta(&mut self, meta: ParseNestedMeta<'_>) -> Result<(), Error> {
        if meta.path.is_ident("attr") {
            let content;
            parenthesized!(content in meta.input);
            self.attrs = content.parse_terminated(Meta::parse, Token![,])?;
            Ok(())
        } else if meta.path.is_ident("omit_bounds") {
            self.omit_bounds = Some(meta.path);
            Ok(())
        } else if meta.path.is_ident("with") {
            meta.input.parse::<Token![=]>()?;
            self.with = Some(meta.input.parse::<Type>()?);
            Ok(())
        } else if meta.path.is_ident("getter") {
            meta.input.parse::<Token![=]>()?;
            self.getter = Some(meta.input.parse::<Path>()?);
            Ok(())
        } else if meta.path.is_ident("niche") {
            let niche = if meta.input.is_empty() {
                Niche::Default
            } else {
                meta.input.parse::<Token![=]>()?;

                Niche::Type(Box::new(meta.input.parse::<Type>()?))
            };

            self.niches.push(niche);

            Ok(())
        } else {
            Err(meta.error("unrecognized rkyv arguments"))
        }
    }

    pub fn parse(
        attributes: &Attributes,
        input: &Field,
    ) -> Result<Self, Error> {
        let mut result = Self::default();

        for attr in input.attrs.iter() {
            if attr.path().is_ident("rkyv") {
                attr.parse_nested_meta(|meta| result.parse_meta(meta))?;
            }
        }

        if result.getter.is_some() && attributes.remote.is_none() {
            return Err(Error::new_spanned(
                result.getter,
                "getters may only be used with remote derive",
            ));
        }

        Ok(result)
    }

    pub fn archive_bound(
        &self,
        rkyv_path: &Path,
        field: &Field,
    ) -> Option<WherePredicate> {
        if self.omit_bounds.is_some() {
            return None;
        }

        let ty = &field.ty;
        if let Some(with) = &self.with {
            Some(parse_quote! {
                #with: #rkyv_path::with::ArchiveWith<#ty>
            })
        } else {
            Some(parse_quote! {
                #ty: #rkyv_path::Archive
            })
        }
    }

    pub fn serialize_bound(
        &self,
        rkyv_path: &Path,
        field: &Field,
    ) -> Option<WherePredicate> {
        if self.omit_bounds.is_some() {
            return None;
        }

        let ty = &field.ty;
        if let Some(with) = &self.with {
            Some(parse_quote! {
                #with: #rkyv_path::with::SerializeWith<#ty, __S>
            })
        } else {
            Some(parse_quote! {
                #ty: #rkyv_path::Serialize<__S>
            })
        }
    }

    pub fn deserialize_bound(
        &self,
        rkyv_path: &Path,
        field: &Field,
    ) -> Option<WherePredicate> {
        if self.omit_bounds.is_some() {
            return None;
        }

        let archived = self.archived(rkyv_path, field);

        let ty = &field.ty;
        if let Some(with) = &self.with {
            Some(parse_quote! {
                #with: #rkyv_path::with::DeserializeWith<#archived, #ty, __D>
            })
        } else {
            Some(parse_quote! {
                #archived: #rkyv_path::Deserialize<#ty, __D>
            })
        }
    }

    fn archive_item(
        &self,
        rkyv_path: &Path,
        field: &Field,
        name: &str,
        with_name: &str,
    ) -> TokenStream {
        let ty = &field.ty;
        if let Some(with) = &self.with {
            let ident = Ident::new(with_name, Span::call_site());
            quote! {
                <#with as #rkyv_path::with::ArchiveWith<#ty>>::#ident
            }
        } else {
            let ident = Ident::new(name, Span::call_site());
            quote! {
                <#ty as #rkyv_path::Archive>::#ident
            }
        }
    }

    pub fn archived(&self, rkyv_path: &Path, field: &Field) -> TokenStream {
        self.archive_item(rkyv_path, field, "Archived", "Archived")
    }

    pub fn resolver(&self, rkyv_path: &Path, field: &Field) -> TokenStream {
        self.archive_item(rkyv_path, field, "Resolver", "Resolver")
    }

    pub fn resolve(&self, rkyv_path: &Path, field: &Field) -> TokenStream {
        self.archive_item(rkyv_path, field, "resolve", "resolve_with")
    }

    pub fn serialize(&self, rkyv_path: &Path, field: &Field) -> TokenStream {
        let ty = &field.ty;
        if let Some(with) = &self.with {
            quote! {
                <
                    #with as #rkyv_path::with::SerializeWith<#ty, __S>
                >::serialize_with
            }
        } else {
            quote! {
                <#ty as #rkyv_path::Serialize<__S>>::serialize
            }
        }
    }

    pub fn deserialize(&self, rkyv_path: &Path, field: &Field) -> TokenStream {
        let ty = &field.ty;
        let archived = self.archived(rkyv_path, field);

        if let Some(with) = &self.with {
            quote! {
                <
                    #with as #rkyv_path::with::DeserializeWith<
                        #archived,
                        #ty,
                        __D,
                    >
                >::deserialize_with
            }
        } else {
            quote! {
                <#archived as #rkyv_path::Deserialize<#ty, __D>>::deserialize
            }
        }
    }

    pub fn access_field(
        &self,
        this: &Ident,
        member: &impl ToTokens,
    ) -> TokenStream {
        if let Some(ref getter) = self.getter {
            quote! { ::core::borrow::Borrow::borrow(&#getter(#this)) }
        } else {
            quote! { &#this.#member }
        }
    }

    pub fn metas(&self) -> TokenStream {
        let mut result = TokenStream::new();

        #[cfg(feature = "bytecheck")]
        if self.omit_bounds.is_some() {
            result.extend(quote! { #[bytecheck(omit_bounds)] });
        }

        for attr in self.attrs.iter() {
            result.extend(quote! { #[#attr] });
        }

        result
    }
}

#[derive(Default)]
pub struct VariantAttributes {
    pub other: Option<Path>,
}

impl VariantAttributes {
    fn parse_meta(&mut self, meta: ParseNestedMeta<'_>) -> Result<(), Error> {
        if meta.path.is_ident("other") {
            self.other = Some(meta.path);
            Ok(())
        } else {
            Err(meta.error("unrecognized rkyv arguments"))
        }
    }

    pub fn parse(
        attributes: &Attributes,
        input: &Variant,
    ) -> Result<Self, Error> {
        let mut result = Self::default();

        for attr in input.attrs.iter() {
            if attr.path().is_ident("rkyv") {
                attr.parse_nested_meta(|meta| result.parse_meta(meta))?;
            }
        }

        if result.other.is_some() {
            if attributes.remote.is_none() {
                return Err(Error::new_spanned(
                    result.other,
                    "`#[rkyv(other)]` may only be used with remote derive",
                ));
            } else if !matches!(input.fields, Fields::Unit) {
                return Err(Error::new_spanned(
                    result.other,
                    "`#[rkyv(other)]` may only be used on unit variants",
                ));
            }
        }

        Ok(result)
    }
}

pub enum Niche {
    Type(Box<Type>),
    Default,
}

impl Niche {
    pub fn to_tokens(&self, rkyv_path: &Path) -> TokenStream {
        match self {
            Niche::Type(ty) => quote!(#ty),
            Niche::Default => quote! {
                #rkyv_path::niche::niching::DefaultNiche
            },
        }
    }
}

impl PartialEq for Niche {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Niche::Type(ty1), Niche::Type(ty2)) => {
                if let (Type::Path(ty1), Type::Path(ty2)) = (&**ty1, &**ty2) {
                    ty1.path.get_ident() == ty2.path.get_ident()
                } else {
                    false
                }
            }
            (Niche::Type(ty), Niche::Default)
            | (Niche::Default, Niche::Type(ty)) => {
                if let Type::Path(ty) = &**ty {
                    match ty.path.get_ident() {
                        Some(ident) => ident == "DefaultNiche",
                        None => false,
                    }
                } else {
                    false
                }
            }
            (Niche::Default, Niche::Default) => true,
        }
    }
}
