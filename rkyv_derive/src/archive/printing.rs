use quote::format_ident;
use syn::{
    parse_quote, DeriveInput, Error, Ident, Meta, Path, Type, Visibility,
};

use crate::{attributes::Attributes, util::strip_raw};

pub struct Printing {
    pub rkyv_path: Path,
    pub vis: Visibility,
    pub name: Ident,
    pub archived_name: Ident,
    pub archived_type: Type,
    pub resolver_name: Ident,
    pub archived_metas: Vec<Meta>,
}

impl Printing {
    pub fn new(
        input: &DeriveInput,
        attributes: &Attributes,
    ) -> Result<Self, Error> {
        let name = input.ident.clone();
        let (_, ty_generics, _) = input.generics.split_for_impl();

        let rkyv_path = attributes
            .crate_path
            .clone()
            .unwrap_or_else(|| parse_quote! { ::rkyv });

        let base_name = strip_raw(&name);
        let mut archived_name = attributes
            .archived
            .clone()
            .unwrap_or_else(|| format_ident!("Archived{}", base_name));

        // This makes it so when you do "goto definition" on an `ArchivedType`,
        // it will go to the definition of `Type` instead of going to definition
        // of the `Archived` derive proc macro
        archived_name.set_span(name.span());

        let archived_type = attributes
            .as_type
            .clone()
            .unwrap_or_else(|| parse_quote! { #archived_name #ty_generics });
        let resolver_name = attributes
            .resolver
            .clone()
            .unwrap_or_else(|| format_ident!("{}Resolver", base_name));

        #[cfg(not(feature = "bytecheck"))]
        let archived_metas = attributes.metas.clone();
        #[cfg(feature = "bytecheck")]
        let archived_metas = {
            let mut result = attributes.metas.clone();
            result.push(parse_quote! {
                derive(#rkyv_path::bytecheck::CheckBytes)
            });
            result.push(parse_quote! {
                bytecheck(crate = #rkyv_path::bytecheck)
            });
            if let Some(attrs) = &attributes.bytecheck {
                result.push(parse_quote! { bytecheck(#attrs) });
            }
            result
        };

        Ok(Self {
            rkyv_path,
            vis: input.vis.clone(),
            name,
            archived_name,
            archived_type,
            resolver_name,
            archived_metas,
        })
    }
}
