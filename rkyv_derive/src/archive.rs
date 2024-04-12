mod r#enum;
mod printing;
mod r#struct;

use core::fmt::Display;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Field, Ident, Meta};

use crate::attributes::Attributes;

pub fn derive(input: &mut DeriveInput) -> Result<TokenStream, Error> {
    let attributes = Attributes::parse(input)?;
    derive_archive_impl(input, &attributes)
}

fn field_archive_attrs(
    field: &Field,
) -> impl '_ + Iterator<Item = &TokenStream> {
    field.attrs.iter().filter_map(|attr| {
        if let Meta::List(list) = &attr.meta {
            if list.path.is_ident("archive_attr") {
                Some(&list.tokens)
            } else {
                None
            }
        } else {
            None
        }
    })
}

fn archived_doc(name: &Ident) -> String {
    format!("An archived [`{}`]", name)
}

fn resolver_doc(name: &Ident) -> String {
    format!("The resolver for an archived [`{}`]", name)
}

fn struct_field_doc(name: &Ident, field_name: &impl Display) -> String {
    format!("The archived counterpart of [`{}::{}`]", name, field_name)
}

fn variant_doc(name: &Ident, variant_name: &Ident) -> String {
    format!("The archived counterpart of [`{}::{}`]", name, variant_name)
}

fn enum_field_doc(
    name: &Ident,
    variant_name: &Ident,
    field_name: &impl Display,
) -> String {
    format!(
        "The archived counterpart of [`{}::{}::{}`]",
        name, variant_name, field_name
    )
}

fn resolver_variant_doc(name: &Ident, variant_name: &Ident) -> String {
    format!("The resolver for [`{}::{}`]", name, variant_name)
}

fn enum_resolver_field_doc(
    name: &Ident,
    variant_name: &Ident,
    field_name: &impl Display,
) -> String {
    format!(
        "The resolver for [`{}::{}::{}`]",
        name, variant_name, field_name
    )
}

fn derive_archive_impl(
    input: &mut DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    let where_clause = input.generics.make_where_clause();
    if let Some(ref bounds) = attributes.archive_bounds {
        for bound in bounds {
            where_clause.predicates.push(bound.clone());
        }
    }

    let printing = printing::Printing::new(input, attributes)?;

    let (archive_types, archive_impls) = match input.data {
        Data::Struct(_) => r#struct::impl_struct(input, attributes, &printing)?,
        Data::Enum(_) => r#enum::impl_enum(input, attributes, &printing)?,
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Archive cannot be derived for unions",
            ))
        }
    };

    let rkyv_path = &printing.rkyv_path;

    Ok(quote! {
        #archive_types

        #[automatically_derived]
        const _: () = {
            use core::marker::PhantomData;
            use #rkyv_path::{out_field, Archive, Archived};

            #archive_impls
        };
    })
}
