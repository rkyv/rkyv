use quote::ToTokens;
use syn::{DeriveInput, Error, Ident, LitStr, Meta, Path, WherePredicate, parse::Parse, punctuated::Punctuated, Token, AttrStyle, meta::ParseNestedMeta, parenthesized};

#[derive(Default)]
pub struct Attributes {
    pub archive_as: Option<LitStr>,
    pub archived: Option<Ident>,
    pub resolver: Option<Ident>,
    pub attrs: Vec<Meta>,
    pub compares: Option<Punctuated<Path, Token![,]>>,
    pub archive_bounds: Option<Punctuated<WherePredicate, Token![,]>>,
    pub serialize_bounds: Option<Punctuated<WherePredicate, Token![,]>>,
    pub deserialize_bounds: Option<Punctuated<WherePredicate, Token![,]>>,
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
            format!("{} already specified", name),
        ))
    }
}

fn parse_archive_attributes(
    attributes: &mut Attributes,
    meta: ParseNestedMeta<'_>,
) -> Result<(), Error> {
    if meta.path.is_ident("check_bytes") {
        if !meta.input.is_empty() && !meta.input.peek(Token![,]) {
            return Err(meta.error("check_bytes argument must be a path"));
        }

        try_set_attribute(
            &mut attributes.check_bytes,
            meta.path,
            "check_bytes",
        )
    } else if meta.path.is_ident("copy_safe") {
        if !meta.input.is_empty() && !meta.input.peek(Token![,]) {
            return Err(meta.error("copy_safe argument must be a path"));
        }

        try_set_attribute(
            &mut attributes.copy_safe,
            meta.path,
            "copy_safe",
        )
    } else if meta.path.is_ident("compare") {
        let traits;
        parenthesized!(traits in meta.input);
        let traits = traits.parse_terminated(Path::parse, Token![,])?;
        try_set_attribute(
            &mut attributes.compares,
            traits,
            "compare",
        )
    } else if meta.path.is_ident("archive_bounds") {
        let bounds;
        parenthesized!(bounds in meta.input);
        let clauses = bounds.parse_terminated(WherePredicate::parse, Token![,])?;
        try_set_attribute(
            &mut attributes.archive_bounds,
            clauses,
            "archive_bounds",
        )
    } else if meta.path.is_ident("serialize_bounds") {
        let bounds;
        parenthesized!(bounds in meta.input);
        let clauses = bounds.parse_terminated(WherePredicate::parse, Token![,])?;
        try_set_attribute(
            &mut attributes.serialize_bounds,
            clauses,
            "serialize_bounds",
        )
    } else if meta.path.is_ident("deserialize_bounds") {
        let bounds;
        parenthesized!(bounds in meta.input);
        let clauses = bounds.parse_terminated(WherePredicate::parse, Token![,])?;
        try_set_attribute(
            &mut attributes.deserialize_bounds,
            clauses,
            "deserialize_bounds",
        )
    } else if meta.path.is_ident("archived") {
        try_set_attribute(
            &mut attributes.archived,
            meta.value()?.parse()?,
            "archived",
        )
    } else if meta.path.is_ident("resolver") {
        try_set_attribute(
            &mut attributes.resolver,
            meta.value()?.parse()?,
            "resolver",
        )
    } else if meta.path.is_ident("as") {
        try_set_attribute(
            &mut attributes.archive_as,
            meta.value()?.parse()?,
            "as",
        )
    } else if meta.path.is_ident("crate") {
        try_set_attribute(
            &mut attributes.rkyv_path,
            meta.value()?.parse()?,
            "crate",
        )
    } else {
        Err(Error::new_spanned(meta.path, "unrecognized archive argument"))
    }
}

pub fn parse_attributes(input: &DeriveInput) -> Result<Attributes, Error> {
    let mut result = Attributes::default();
    for attr in input.attrs.iter() {
        if !matches!(attr.style, AttrStyle::Outer) {
            continue;
        }

        if attr.path().is_ident("archive") {
            attr.parse_nested_meta(|nested| {
                parse_archive_attributes(&mut result, nested)
            })?;
        } else if attr.path().is_ident("archive_attr") {
            result.attrs.extend(attr
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                .into_iter());
        }
    }

    Ok(result)
}
