use quote::ToTokens;
use syn::{AttrStyle, DeriveInput, Error, Ident, Lit, LitStr, Meta, MetaList, NestedMeta, Path};
use crate::repr::ReprAttr;

#[derive(Default)]
pub struct Attributes {
    pub attrs: Vec<Meta>,
    pub copy: Option<Path>,
    pub repr: Option<ReprAttr>,
    pub derives: Option<MetaList>,
    pub compares: Option<(Path, Vec<Path>)>,
    pub serialize_bound: Option<LitStr>,
    pub deserialize_bound: Option<LitStr>,
    pub archived: Option<Ident>,
    pub archived_repr: Option<ReprAttr>,
    pub resolver: Option<Ident>,
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
            &format!("{} already specified", name),
        ))
    }
}

fn parse_archive_attributes(attributes: &mut Attributes, meta: &Meta) -> Result<(), Error> {
    match meta {
        Meta::Path(path) => {
            if path.is_ident("copy") {
                try_set_attribute(&mut attributes.copy, path.clone(), "copy")
            } else {
                Err(Error::new_spanned(path, "unrecognized archive argument"))
            }
        }
        Meta::List(list) => {
            if list.path.is_ident("compare") {
                if attributes.compares.is_none() {
                    let mut compares = Vec::new();
                    for compare in list.nested.iter() {
                        if let NestedMeta::Meta(Meta::Path(path)) = compare {
                            compares.push(path.clone());
                        } else {
                            return Err(Error::new_spanned(
                                compare,
                                "compare arguments must be compare traits to derive",
                            ));
                        }
                    }
                    attributes.compares = Some((list.path.clone(), compares));
                    Ok(())
                } else {
                    Err(Error::new_spanned(list, "compares already specified"))
                }
            } else if list.path.is_ident("bound") {
                for bound in list.nested.iter() {
                    if let NestedMeta::Meta(Meta::NameValue(name_value)) = bound {
                        if let Lit::Str(ref lit_str) = name_value.lit {
                            if name_value.path.is_ident("serialize") {
                                if attributes.serialize_bound.is_none() {
                                    attributes.serialize_bound = Some(lit_str.clone());
                                } else {
                                    return Err(Error::new_spanned(
                                        bound,
                                        "serialize bound already specified",
                                    ));
                                }
                            } else if name_value.path.is_ident("deserialize") {
                                if attributes.deserialize_bound.is_none() {
                                    attributes.deserialize_bound = Some(lit_str.clone());
                                } else {
                                    return Err(Error::new_spanned(
                                        bound,
                                        "serialize bound already specified",
                                    ));
                                }
                            } else {
                                return Err(Error::new_spanned(
                                    bound,
                                    "bound must be either serialize or deserialize",
                                ));
                            }
                        } else {
                            return Err(Error::new_spanned(
                                bound,
                                "bound arguments must be a string",
                            ));
                        }
                    } else {
                        return Err(Error::new_spanned(
                            bound,
                            "bound arguments must be serialize or deserialize bounds to apply",
                        ));
                    }
                }
                Ok(())
            } else if list.path.is_ident("repr") {
                if list.nested.len() != 1 {
                    Err(Error::new_spanned(list, "repr must have exactly one argument"))
                } else {
                    if let Some(NestedMeta::Meta(Meta::Path(path))) = list.nested.first() {
                        try_set_attribute(
                            &mut attributes.archived_repr,
                            ReprAttr::try_from_path(path)?,
                            "repr"
                        )
                    } else {
                        Err(Error::new_spanned(list.nested.first(), "invalid repr argument"))
                    }
                }
            } else {
                Err(Error::new_spanned(
                    &list.path,
                    "unrecognized archive argument",
                ))
            }
        }
        Meta::NameValue(meta) => {
            if meta.path.is_ident("archived") {
                if let Lit::Str(ref lit_str) = meta.lit {
                    try_set_attribute(
                        &mut attributes.archived,
                        Ident::new(&lit_str.value(), lit_str.span()),
                        "archived",
                    )
                } else {
                    Err(Error::new_spanned(meta, "archived must be a string"))
                }
            } else if meta.path.is_ident("resolver") {
                if let Lit::Str(ref lit_str) = meta.lit {
                    try_set_attribute(
                        &mut attributes.resolver,
                        Ident::new(&lit_str.value(), lit_str.span()),
                        "resolver",
                    )
                } else {
                    Err(Error::new_spanned(meta, "resolver must be a string"))
                }
            } else {
                Err(Error::new_spanned(meta, "unrecognized archive argument"))
            }
        }
    }
}

pub fn parse_attributes(input: &DeriveInput) -> Result<Attributes, Error> {
    let mut result = Attributes::default();
    for attr in input.attrs.iter() {
        if let AttrStyle::Outer = attr.style {
            if let Ok(Meta::List(list)) = attr.parse_meta() {
                if list.path.is_ident("archive") {
                    for nested in list.nested.iter() {
                        if let NestedMeta::Meta(meta) = nested {
                            parse_archive_attributes(&mut result, meta)?;
                        } else {
                            return Err(Error::new_spanned(
                                nested,
                                "archive arguments must be metas"
                            ));
                        }
                    }
                } else if list.path.is_ident("archive_attr") {
                    for nested in list.nested.iter() {
                        if let NestedMeta::Meta(meta) = nested {
                            result.attrs.push(meta.clone());
                        } else {
                            return Err(Error::new_spanned(
                                nested,
                                "archive_attr arguments must be metas"
                            ));
                        }
                    }
                } else if list.path.is_ident("repr") {
                    if list.nested.len() != 1 {
                        return Err(Error::new_spanned(list, "repr must have exactly one argument"));
                    } else {
                        if let Some(NestedMeta::Meta(Meta::Path(path))) = list.nested.first() {
                            try_set_attribute(
                                &mut result.repr,
                                ReprAttr::try_from_path(path)?,
                                "repr"
                            )?;
                        } else {
                            return Err(Error::new_spanned(
                                list.nested.first(),
                                "invalid repr argument"
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(result)
}
