use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput,
    Error, Fields, Generics, Ident, Index, Path, WhereClause,
};

use crate::{
    archive::printing::Printing,
    attributes::{Attributes, FieldAttributes},
};

pub fn derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = Attributes::parse(&input)?;
    derive_deserialize_impl(input, &attributes)
}

fn derive_deserialize_impl(
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
    if let Some(ref bounds) = attributes.deserialize_bounds {
        for bound in bounds {
            where_clause.predicates.push(bound.clone());
        }
    }

    let mut impl_input_params = Punctuated::default();
    impl_input_params
        .push(parse_quote! { __D: #rkyv_path::rancor::Fallible + ?Sized });
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

    let mut deserialize_where = where_clause.clone();

    if let Some(ref remote) = attributes.remote {
        let printing = Printing::new(&input, attributes)?;

        let body = generate_deserialize_body(
            &input,
            attributes,
            &mut deserialize_where,
            &rkyv_path,
            printing.archived_name,
            name,
        )?;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics
                #rkyv_path::with::DeserializeWith<
                    <
                        #name #ty_generics as
                            #rkyv_path::with::ArchiveWith<#remote>
                    >::Archived,
                    #remote,
                    __D,
                >
                for #name #ty_generics
            #deserialize_where
            {
                fn deserialize_with(
                    field: &<
                        #name #ty_generics as
                            #rkyv_path::with::ArchiveWith<#remote>
                    >::Archived,
                    deserializer: &mut __D,
                ) -> ::core::result::Result<
                    #remote,
                    <__D as #rkyv_path::rancor::Fallible>::Error,
                > {
                    let __this = field;
                    #body.map(<#remote as From<#name #ty_generics>>::from)
                }
            }
        })
    } else {
        let body = generate_deserialize_body(
            &input,
            attributes,
            &mut deserialize_where,
            &rkyv_path,
            Ident::new("Self", Span::call_site()),
            name,
        )?;

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics #rkyv_path::Deserialize<#name #ty_generics, __D>
                for #rkyv_path::Archived<#name #ty_generics>
            #deserialize_where
            {
                fn deserialize(
                    &self,
                    deserializer: &mut __D,
                ) -> ::core::result::Result<
                    #name #ty_generics,
                    <__D as #rkyv_path::rancor::Fallible>::Error,
                > {
                    let __this = self;
                    #body
                }
            }
        })
    }
}

fn generate_deserialize_body(
    input: &DeriveInput,
    attributes: &Attributes,
    deserialize_where: &mut WhereClause,
    rkyv_path: &Path,
    self_type: Ident,
    return_type: &Ident,
) -> Result<TokenStream, Error> {
    let this = Ident::new("__this", Span::call_site());
    let body = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let deserialize_fields = fields
                    .named
                    .iter()
                    .map(|field| {
                        let field_attrs =
                            FieldAttributes::parse(attributes, field)?;

                        deserialize_where.predicates.extend(
                            field_attrs.archive_bound(rkyv_path, field),
                        );
                        deserialize_where.predicates.extend(
                            field_attrs.deserialize_bound(rkyv_path, field),
                        );

                        let name = &field.ident;
                        let deserialize =
                            field_attrs.deserialize(rkyv_path, field);
                        Ok(quote! {
                            #name: #deserialize(&#this.#name, deserializer)?
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                quote! { #return_type { #(#deserialize_fields,)* } }
            }
            Fields::Unnamed(ref fields) => {
                let deserialize_fields = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, field)| {
                        let field_attrs =
                            FieldAttributes::parse(attributes, field)?;

                        deserialize_where.predicates.extend(
                            field_attrs.archive_bound(rkyv_path, field),
                        );
                        deserialize_where.predicates.extend(
                            field_attrs.deserialize_bound(rkyv_path, field),
                        );

                        let index = Index::from(i);
                        let deserialize =
                            field_attrs.deserialize(rkyv_path, field);
                        Ok(quote! {
                            #deserialize(&#this.#index, deserializer)?
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                quote! { #return_type(#(#deserialize_fields,)*) }
            }
            Fields::Unit => quote! { #return_type },
        },
        Data::Enum(ref data) => {
            let deserialize_variants = data
                .variants
                .iter()
                .map(|v| {
                    let variant = &v.ident;
                    match v.fields {
                        Fields::Named(ref fields) => {
                            let bindings = fields.named.iter().map(|field| {
                                let name = &field.ident;
                                quote! { #name }
                            });
                            let fields = fields
                                .named
                                .iter()
                                .map(|field| {
                                    let field_attrs = FieldAttributes::parse(
                                        attributes, field,
                                    )?;

                                    deserialize_where.predicates.extend(
                                        field_attrs
                                            .archive_bound(rkyv_path, field),
                                    );
                                    deserialize_where.predicates.extend(
                                        field_attrs.deserialize_bound(
                                            rkyv_path, field,
                                        ),
                                    );

                                    let name = &field.ident;
                                    let deserialize = field_attrs
                                        .deserialize(rkyv_path, field);
                                    Ok(quote! {
                                        #name: #deserialize(
                                            #name,
                                            deserializer,
                                        )?
                                    })
                                })
                                .collect::<Result<Vec<_>, Error>>()?;
                            Ok(quote! {
                                #self_type::#variant {
                                    #(#bindings,)*..
                                } => #return_type::#variant { #(#fields,)* }
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

                                    deserialize_where.predicates.extend(
                                        field_attrs
                                            .archive_bound(rkyv_path, field),
                                    );
                                    deserialize_where.predicates.extend(
                                        field_attrs.deserialize_bound(
                                            rkyv_path, field,
                                        ),
                                    );

                                    let binding = Ident::new(
                                        &format!("_{i}"),
                                        field.span(),
                                    );
                                    let deserialize = field_attrs
                                        .deserialize(rkyv_path, field);
                                    Ok(quote! {
                                        #deserialize(
                                            #binding,
                                            deserializer,
                                        )?
                                    })
                                })
                                .collect::<Result<Vec<_>, Error>>()?;
                            Ok(quote! {
                                #self_type::#variant(
                                    #(#bindings,)*..
                                ) => #return_type::#variant(#(#fields,)*)
                            })
                        }
                        Fields::Unit => Ok(quote! {
                            #self_type::#variant => #return_type::#variant
                        }),
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?;

            quote! {
                match __this {
                    #(#deserialize_variants,)*
                }
            }
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Deserialize cannot be derived for unions",
            ))
        }
    };

    Ok(quote! { ::core::result::Result::Ok(#body) })
}
