use quote::quote;
use syn::{
    parse_quote, spanned::Spanned as _, Attribute, DeriveInput, Error, Ident,
    LitStr, Meta, Path, Type,
};

use crate::{attributes::Attributes, util::strip_raw};

pub struct Printing {
    pub rkyv_path: Path,
    pub archived_name: Ident,
    pub archived_type: Type,
    pub resolver_name: Ident,
    pub archive_attrs: Vec<Attribute>,
}

impl Printing {
    pub fn new(
        input: &DeriveInput,
        attributes: &Attributes,
    ) -> Result<Self, Error> {
        let name = &input.ident;

        let rkyv_path = attributes
            .crate_path
            .clone()
            .unwrap_or_else(|| parse_quote! { ::rkyv });

        if let Some(ref archive_as) = attributes.archive_as {
            if let Some(ref ident) = attributes.archived {
                return Err(Error::new_spanned(
                    ident,
                    "archived = \"...\" may not be used with as = \"...\" \
                     because no type is generated",
                ));
            }
            if let Some(first) = attributes.attrs.first() {
                return Err(Error::new_spanned(
                    first,
                    format!(
                        "attributes may not be used with as = \"...\"\nplace \
                         any attributes on the archived type ({}) instead",
                        archive_as.value(),
                    ),
                ));
            }
        }

        let archived_name = attributes.archived.as_ref().map_or_else(
            || Ident::new(&format!("Archived{}", strip_raw(name)), name.span()),
            |value| value.clone(),
        );

        let resolver_name = attributes.resolver.as_ref().map_or_else(
            || Ident::new(&format!("{}Resolver", strip_raw(name)), name.span()),
            |value| value.clone(),
        );

        let archived_type = attributes.archive_as.as_ref().map_or_else(
            || {
                let (_, ty_generics, _) = input.generics.split_for_impl();
                Ok(parse_quote! { #archived_name #ty_generics })
            },
            |lit| lit.parse::<Type>(),
        )?;

        let derive_check_bytes = if attributes.check_bytes.is_some()
            && cfg!(feature = "bytecheck")
        {
            let path = quote!(#rkyv_path::bytecheck).to_string();
            let path_lit_str = LitStr::new(&path, rkyv_path.span());
            let mut result = vec![
                parse_quote! { #[derive(#rkyv_path::bytecheck::CheckBytes)] },
                parse_quote! { #[check_bytes(crate = #path_lit_str)] },
            ];

            if let Meta::List(check_bytes) =
                attributes.check_bytes.as_ref().unwrap()
            {
                result.push(parse_quote! { #[#check_bytes] });
            }

            result
        } else {
            Vec::new()
        };

        let archive_attrs = derive_check_bytes
            .into_iter()
            .chain(
                attributes
                    .attrs
                    .iter()
                    .map::<Attribute, _>(|d| parse_quote! { #[#d] }),
            )
            .collect();

        Ok(Self {
            rkyv_path,
            archived_name,
            archived_type,
            resolver_name,
            archive_attrs,
        })
    }
}
