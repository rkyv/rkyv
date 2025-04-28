use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput,
    Error, Fields, Generics, Ident, Index, Path, WhereClause,
};

use crate::{
    attributes::{Attributes, FieldAttributes, VariantAttributes},
    util::{strip_generics_from_path, strip_raw},
};

pub fn derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = Attributes::parse(&input)?;
    derive_serialize_impl(input, &attributes)
}

fn derive_serialize_impl(
    mut input: DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    let rkyv_path = attributes.crate_path();

    let where_clause = input.generics.make_where_clause();
    if let Some(ref bounds) = attributes.archive_bounds {
        for bound in bounds {
            where_clause.predicates.push(bound.clone());
        }
    }
    if let Some(ref bounds) = attributes.serialize_bounds {
        for bound in bounds {
            where_clause.predicates.push(bound.clone());
        }
    }

    let mut impl_input_params = Punctuated::default();
    impl_input_params
        .push(parse_quote! { __S: #rkyv_path::rancor::Fallible + ?Sized });
    for param in input.generics.params.iter() {
        impl_input_params.push(param.clone());
    }
    let impl_input_generics = Generics {
        lt_token: Some(Default::default()),
        params: impl_input_params,
        gt_token: Some(Default::default()),
        where_clause: input.generics.where_clause.clone(),
    };

    let name = &input.ident;
    let (impl_generics, ..) = impl_input_generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let resolver = attributes.resolver.as_ref().map_or_else(
        || Ident::new(&format!("{}Resolver", strip_raw(name)), name.span()),
        |value| value.clone(),
    );

    let mut serialize_where = where_clause.clone();

    if let Some(ref remote) = attributes.remote {
        let body = generate_serialize_body(
            &input,
            attributes,
            &mut serialize_where,
            &rkyv_path,
            resolver,
            strip_generics_from_path(remote.clone()),
        )?;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #rkyv_path::with::SerializeWith<#remote, __S>
                for #name #ty_generics
            #serialize_where
            {
                fn serialize_with(
                    field: &#remote,
                    serializer: &mut __S,
                ) -> ::core::result::Result<
                    <Self as #rkyv_path::with::ArchiveWith<#remote>>::Resolver,
                    <__S as #rkyv_path::rancor::Fallible>::Error,
                > {
                    let __this = field;
                    #body
                }
            }
        })
    } else {
        let body = generate_serialize_body(
            &input,
            attributes,
            &mut serialize_where,
            &rkyv_path,
            resolver,
            parse_quote!(#name),
        )?;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #rkyv_path::Serialize<__S>
                for #name #ty_generics
            #serialize_where
            {
                fn serialize(
                    &self,
                    serializer: &mut __S,
                ) -> ::core::result::Result<
                    <Self as #rkyv_path::Archive>::Resolver,
                    <__S as #rkyv_path::rancor::Fallible>::Error,
                > {
                    let __this = self;
                    #body
                }
            }
        })
    }
}

fn generate_serialize_body(
    input: &DeriveInput,
    attributes: &Attributes,
    serialize_where: &mut WhereClause,
    rkyv_path: &Path,
    resolver: Ident,
    name: Path,
) -> Result<TokenStream, Error> {
    let this = Ident::new("__this", Span::call_site());
    let body = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let resolver_values = fields
                    .named
                    .iter()
                    .map(|field| {
                        let field_attrs =
                            FieldAttributes::parse(attributes, field)?;

                        serialize_where.predicates.extend(
                            field_attrs.serialize_bound(rkyv_path, field),
                        );

                        let name = &field.ident;
                        let access_field =
                            field_attrs.access_field(&this, name);
                        let serialize = field_attrs.serialize(rkyv_path, field);
                        Ok(quote! {
                            #name: #serialize(#access_field, serializer)?
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                quote! { #resolver { #(#resolver_values,)* } }
            }
            Fields::Unnamed(ref fields) => {
                let resolver_values = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, field)| {
                        let field_attrs =
                            FieldAttributes::parse(attributes, field)?;

                        serialize_where.predicates.extend(
                            field_attrs.serialize_bound(rkyv_path, field),
                        );

                        let index = Index::from(i);
                        let access_field =
                            field_attrs.access_field(&this, &index);
                        let serialize = field_attrs.serialize(rkyv_path, field);
                        Ok(quote! { #serialize(#access_field, serializer)? })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                quote! { #resolver(#(#resolver_values,)*) }
            }
            Fields::Unit => quote! { #resolver },
        },
        Data::Enum(ref data) => {
            let mut other: Option<Path> = None;
            let serialize_arms = data
                .variants
                .iter()
                .map(|v| {
                    if let Some(ref other) = other {
                        return Err(Error::new_spanned(
                            other,
                            "Only the very last variant may be denoted with \
                             `#[rkyv(other)]`",
                        ));
                    }
                    let variant_attrs =
                        VariantAttributes::parse(attributes, v)?;
                    let variant = &v.ident;
                    match v.fields {
                        Fields::Named(ref fields) => {
                            let bindings =
                                fields.named.iter().map(|f| &f.ident);
                            let fields = fields
                                .named
                                .iter()
                                .map(|field| {
                                    let field_attrs = FieldAttributes::parse(
                                        attributes, field,
                                    )?;

                                    serialize_where.predicates.extend(
                                        field_attrs
                                            .serialize_bound(rkyv_path, field),
                                    );

                                    let name = &field.ident;
                                    let serialize =
                                        field_attrs.serialize(rkyv_path, field);
                                    Ok(quote! {
                                        #name: #serialize(#name, serializer)?
                                    })
                                })
                                .collect::<Result<Vec<_>, Error>>()?;
                            Ok(quote! {
                                #name::#variant {
                                    #(#bindings,)*..
                                } => #resolver::#variant {
                                    #(#fields,)*
                                }
                            })
                        }
                        Fields::Unnamed(ref fields) => {
                            let bindings =
                                fields.unnamed.iter().enumerate().map(
                                    |(i, f)| {
                                        Ident::new(&format!("_{i}"), f.span())
                                    },
                                );

                            let fields = fields
                                .unnamed
                                .iter()
                                .enumerate()
                                .map(|(i, field)| {
                                    let field_attrs = FieldAttributes::parse(
                                        attributes, field,
                                    )?;

                                    serialize_where.predicates.extend(
                                        field_attrs
                                            .serialize_bound(rkyv_path, field),
                                    );

                                    let binding = Ident::new(
                                        &format!("_{i}"),
                                        field.span(),
                                    );
                                    let serialize =
                                        field_attrs.serialize(rkyv_path, field);
                                    Ok(quote! {
                                        #serialize(#binding, serializer)?
                                    })
                                })
                                .collect::<Result<Vec<_>, Error>>()?;
                            Ok(quote! {
                                #name::#variant(
                                    #(#bindings,)*..
                                ) => #resolver::#variant(#(#fields,)*)
                            })
                        }
                        Fields::Unit => {
                            if variant_attrs.other.is_some() {
                                other = variant_attrs.other;
                                Ok(quote! { _ => #resolver::#variant })
                            } else {
                                Ok(quote! {
                                    #name::#variant => #resolver::#variant
                                })
                            }
                        }
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?;

            quote! {
                match __this {
                    #(#serialize_arms,)*
                }
            }
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Serialize cannot be derived for unions",
            ))
        }
    };

    Ok(quote! { ::core::result::Result::Ok(#body) })
}
