use crate::{repr::Repr, util::PunctuatedExt};
use quote::ToTokens;
use syn::{
    meta::ParseNestedMeta, parenthesized, AttrStyle, DeriveInput, Error, Ident,
    LitStr, Meta, Path, Token,
};

#[derive(Default)]
pub struct Attributes {
    pub archive_as: Option<LitStr>,
    pub archived: Option<Ident>,
    pub resolver: Option<Ident>,
    pub attrs: Vec<Meta>,
    pub archived_repr: Repr,
    pub compares: Option<(Path, Vec<Path>)>,
    pub archive_bound: Option<LitStr>,
    pub serialize_bound: Option<LitStr>,
    pub deserialize_bound: Option<LitStr>,
    pub check_bytes: Option<Path>,
    pub copy_safe: Option<Path>,
    pub rkyv_path: Option<Path>,
    pub rkyv_path_str: Option<LitStr>,
}

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

fn parse_archive_attributes(
    attributes: &mut Attributes,
    meta: ParseNestedMeta<'_>,
) -> Result<(), Error> {
    if meta.path.is_ident("check_bytes") {
        try_set_attribute(&mut attributes.check_bytes, meta.path, "check_bytes")
    } else if meta.path.is_ident("copy_safe") {
        try_set_attribute(&mut attributes.copy_safe, meta.path, "copy_safe")
    } else if meta.path.is_ident("archived") {
        let ident = meta.value()?.parse::<LitStr>()?.parse()?;

        try_set_attribute(&mut attributes.archived, ident, "archived")
    } else if meta.path.is_ident("resolver") {
        let ident = meta.value()?.parse::<LitStr>()?.parse()?;

        try_set_attribute(&mut attributes.resolver, ident, "resolver")
    } else if meta.path.is_ident("as") {
        try_set_attribute(
            &mut attributes.archive_as,
            meta.value()?.parse()?,
            "archive as",
        )
    } else if meta.path.is_ident("crate") {
        let lit_str: LitStr = meta.value()?.parse()?;
        let stream = syn::parse_str(&lit_str.value())?;
        let tokens = crate::serde::respan::respan(stream, lit_str.span());
        let path: Path = syn::parse2(tokens)?;
        try_set_attribute(&mut attributes.rkyv_path, path, "crate")?;
        attributes.rkyv_path_str = Some(lit_str);

        Ok(())
    } else if meta.path.is_ident("compare") {
        if attributes.compares.is_some() {
            let msg = "compares already specified";

            return Err(Error::new_spanned(meta.path, msg));
        }

        let content;
        parenthesized!(content in meta.input);

        let compares = Vec::parse_separated_nonempty::<Token![,]>(&content)?;
        attributes.compares = Some((meta.path, compares));

        Ok(())
    } else if meta.path.is_ident("bound") {
        meta.parse_nested_meta(|nested| {
            let (bound, name) = if nested.path.is_ident("archive") {
                (&mut attributes.archive_bound, "archive bound")
            } else if nested.path.is_ident("serialize") {
                (&mut attributes.serialize_bound, "serialize bound")
            } else if nested.path.is_ident("deserialize") {
                (&mut attributes.deserialize_bound, "deserialize bound")
            } else {
                let msg =
                    "bound must be either archive, serialize, or deserialize";

                return Err(Error::new_spanned(nested.path, msg));
            };

            let lit_str = nested.value()?.parse()?;

            try_set_attribute(bound, lit_str, name)
        })
    } else if meta.path.is_ident("repr") {
        // TODO: remove `archive(repr(...))` syntax
        meta.parse_nested_meta(|nested| {
            attributes.archived_repr.parse_list_meta(nested)
        })
    } else {
        Err(meta.error("unrecognized archive argument"))
    }
}

pub fn parse_attributes(input: &DeriveInput) -> Result<Attributes, Error> {
    let mut result = Attributes::default();

    for attr in input.attrs.iter() {
        let AttrStyle::Outer = attr.style else {
            continue;
        };

        if !(attr.path().is_ident("archive")
            || attr.path().is_ident("archive_attr"))
        {
            continue;
        }

        let Meta::List(ref list) = attr.meta else {
            let msg = "archive and archive_attr may only be structured list attributes";

            return Err(Error::new_spanned(attr, msg));
        };

        if list.path.is_ident("archive") {
            list.parse_nested_meta(|nested| {
                parse_archive_attributes(&mut result, nested)
            })?;
        } else if list.path.is_ident("archive_attr") {
            let metas = list
                .parse_args_with(Vec::parse_separated_nonempty::<Token![,]>)
                .map_err(|e| {
                    Error::new(e.span(), "archive_attr arguments must be metas")
                })?;

            for meta in metas {
                let Meta::List(list) = meta else {
                    result.attrs.push(meta);

                    continue;
                };

                if list.path.is_ident("repr") {
                    list.parse_nested_meta(|nested| {
                        result.archived_repr.parse_list_meta(nested)
                    })?;
                } else {
                    result.attrs.push(Meta::List(list));
                }
            }
        }
    }

    Ok(result)
}
