mod r#enum;
pub mod printing;
mod r#struct;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, Data, DataStruct, DeriveInput, Error, Ident};

use crate::{
    archive::printing::Printing,
    attributes::{Attributes, FieldAttributes},
    util::iter_fields,
};

pub fn derive(input: &mut DeriveInput) -> Result<TokenStream, Error> {
    let attributes = Attributes::parse(input)?;
    derive_archive_impl(input, &attributes)
}

fn archived_doc(name: &Ident) -> String {
    format!("An archived [`{name}`]")
}

fn resolver_doc(name: &Ident) -> String {
    format!("The resolver for an archived [`{name}`]")
}

fn variant_doc(name: &Ident, variant_name: &Ident) -> String {
    format!("The archived counterpart of [`{name}::{variant_name}`]")
}

fn resolver_variant_doc(name: &Ident, variant_name: &Ident) -> String {
    format!("The resolver for [`{name}::{variant_name}`]")
}

fn derive_archive_impl(
    input: &mut DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    let printing = Printing::new(input, attributes)?;

    let where_clause = input.generics.make_where_clause();
    if let Some(ref bounds) = attributes.archive_bounds {
        where_clause.predicates.extend(bounds.iter().cloned());
    }
    for field in iter_fields(&input.data) {
        let field_attrs = FieldAttributes::parse(attributes, field)?;
        where_clause
            .predicates
            .extend(field_attrs.archive_bound(&printing.rkyv_path, field));
    }

    let mut result = match &input.data {
        Data::Struct(DataStruct { fields, .. }) => r#struct::impl_struct(
            &printing,
            &input.generics,
            attributes,
            fields,
        )?,
        Data::Enum(enm) => {
            r#enum::impl_enum(&printing, &input.generics, attributes, enm)?
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Archive cannot be derived for unions",
            ))
        }
    };

    if attributes.as_type.is_none() {
        result
            .extend(impl_auto_trait(input, &printing, attributes, "Portable")?);
    }

    Ok(result)
}

fn impl_auto_trait(
    input: &DeriveInput,
    printing: &Printing,
    attributes: &Attributes,
    trait_name: &str,
) -> Result<TokenStream, Error> {
    let mut generics = input.generics.clone();
    let where_clause = generics.make_where_clause();

    let rkyv_path = &printing.rkyv_path;
    let trait_ident = Ident::new(trait_name, Span::call_site());

    for field in iter_fields(&input.data) {
        let field_attrs = FieldAttributes::parse(attributes, field)?;
        let archived_field_ty = field_attrs.archived(rkyv_path, field);

        where_clause.predicates.push(parse_quote! {
            #archived_field_ty: #rkyv_path::traits::#trait_ident
        });
    }

    let archived_name = &printing.archived_name;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        // SAFETY: These pseudo-auto traits are implemented for the archived
        // type if they are implemented for all of its fields.
        unsafe impl #impl_generics #rkyv_path::traits::#trait_ident
            for #archived_name #ty_generics
        #where_clause
        {}
    })
}
