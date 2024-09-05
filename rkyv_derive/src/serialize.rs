use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DataEnum, DeriveInput, Error, Field, Fields, Generics, Ident, Index, Path
};

use crate::{
    attributes::Attributes,
    util::{is_not_omitted, remote_field_access, serialize, serialize_bound, serialize_remote, serialize_remote_bound, strip_generics_from_path, strip_raw},
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

    let (serialize_impl, serialize_with_impl) = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let mut serialize_where = where_clause.clone();
                for field in fields.named.iter().filter(is_not_omitted) {
                    serialize_where
                        .predicates
                        .push(serialize_bound(&rkyv_path, field)?);

                    if let Some(remote_clause) =
                        serialize_remote_bound(&rkyv_path, field)?
                    {
                        serialize_where.predicates.push(remote_clause);
                    }
                }

                let resolver_values = fields.named.iter().map(|field| {
                    let name = &field.ident;
                    let serialize = serialize(&rkyv_path, field)?;
                    Ok(quote! { #name: #serialize(&self.#name, serializer)? })
                }).collect::<Result<Vec<_>, Error>>()?;

                let serialize_impl = quote! {
                    impl #impl_generics #rkyv_path::Serialize<__S>
                        for #name #ty_generics
                    #serialize_where
                    {
                        fn serialize(
                            &self,
                            serializer: &mut __S
                        ) -> ::core::result::Result<
                            Self::Resolver,
                            <__S as #rkyv_path::rancor::Fallible>::Error,
                        > {
                            Ok(#resolver {
                                #(#resolver_values,)*
                            })
                        }
                    }
                };

                let serialize_with_impl = if let Some(ref remote) = attributes.remote {
                    let resolver_values = fields.named.iter().map(|field| {
                        let name = &field.ident;
                        let serialize = serialize_remote(&rkyv_path, field)?;
                        let field_access = remote_field_access(field, &field.ident)?;
                        Ok(quote! { #name: #serialize(#field_access, serializer)? })
                    }).collect::<Result<Vec<_>, Error>>()?;

                    quote! {
                        #[automatically_derived]
                        impl #impl_generics #rkyv_path::with::SerializeWith<#remote, __S>
                            for #name #ty_generics
                        #serialize_where
                        {
                            fn serialize_with(
                                field: &#remote,
                                serializer: &mut __S
                            ) -> ::core::result::Result<
                                Self::Resolver,
                                <__S as #rkyv_path::rancor::Fallible>::Error,
                            > {
                                ::core::result::Result::Ok(#resolver {
                                    #(#resolver_values,)*
                                })
                            }
                        }
                    }
                } else {
                    TokenStream::new()
                };

                (serialize_impl, serialize_with_impl)
            }
            Fields::Unnamed(ref fields) => {
                let mut serialize_where = where_clause.clone();
                for field in fields.unnamed.iter().filter(is_not_omitted) {
                    serialize_where
                        .predicates
                        .push(serialize_bound(&rkyv_path, field)?);

                    if let Some(remote_clause) =
                        serialize_remote_bound(&rkyv_path, field)?
                    {
                        serialize_where.predicates.push(remote_clause);
                    }
                }

                let resolver_values = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, field)| {
                        let index = Index::from(i);
                        let serialize = serialize(&rkyv_path, field)?;
                        Ok(quote! { #serialize(&self.#index, serializer)? })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                let serialize_impl = quote! {
                    impl #impl_generics #rkyv_path::Serialize<__S>
                        for #name #ty_generics
                    #serialize_where
                    {
                        fn serialize(
                            &self,
                            serializer: &mut __S,
                        ) -> ::core::result::Result<
                            Self::Resolver,
                            <__S as #rkyv_path::rancor::Fallible>::Error,
                        > {
                            Ok(#resolver(
                                #(#resolver_values,)*
                            ))
                        }
                    }
                };

                let serialize_with_impl = if let Some(ref remote) = attributes.remote {
                    let resolver_values = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, field)| {
                            let index = Index::from(i);
                            let serialize = serialize_remote(&rkyv_path, field)?;
                            let field_access = remote_field_access(field, &index)?;
                            Ok(quote! { #serialize(#field_access, serializer)? })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;

                    quote! {
                        #[automatically_derived]
                        impl #impl_generics #rkyv_path::with::SerializeWith<#remote, __S>
                            for #name #ty_generics
                        #serialize_where
                        {
                            fn serialize_with(
                                field: &#remote,
                                serializer: &mut __S,
                            ) -> ::core::result::Result<
                                Self::Resolver,
                                <__S as #rkyv_path::rancor::Fallible>::Error,
                            > {
                                Ok(#resolver(
                                    #(#resolver_values,)*
                                ))
                            }
                        }
                    }
                } else {
                    TokenStream::new()
                };

                (serialize_impl, serialize_with_impl)
            }
            Fields::Unit => {
                let serialize_impl = quote! {
                    impl #impl_generics #rkyv_path::Serialize<__S>
                        for #name #ty_generics
                    #where_clause
                    {
                        fn serialize(
                            &self,
                            serializer: &mut __S,
                        ) -> ::core::result::Result<
                            Self::Resolver,
                            <__S as #rkyv_path::rancor::Fallible>::Error,
                        > {
                            Ok(#resolver)
                        }
                    }
                };

                let serialize_with_impl = if let Some(ref remote) =
                    attributes.remote
                {
                    quote! {
                        #[automatically_derived]
                        impl #impl_generics #rkyv_path::with::SerializeWith<#remote, __S>
                            for #name #ty_generics
                        #where_clause
                        {
                            fn serialize_with(
                                field: &#remote,
                                serializer: &mut __S,
                            ) -> ::core::result::Result<
                                Self::Resolver,
                                <__S as #rkyv_path::rancor::Fallible>::Error,
                            > {
                                Ok(#resolver)
                            }
                        }
                    }
                } else {
                    TokenStream::new()
                };

                (serialize_impl, serialize_with_impl)
            }
        },
        Data::Enum(ref data) => {
            let mut serialize_where = where_clause.clone();
            for variant in data.variants.iter() {
                match variant.fields {
                    Fields::Named(ref fields) => {
                        for field in fields.named.iter().filter(is_not_omitted)
                        {
                            serialize_where
                                .predicates
                                .push(serialize_bound(&rkyv_path, field)?);

                            if let Some(remote_clause) =
                                serialize_remote_bound(&rkyv_path, field)?
                            {
                                serialize_where.predicates.push(remote_clause);
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        for field in
                            fields.unnamed.iter().filter(is_not_omitted)
                        {
                            serialize_where
                                .predicates
                                .push(serialize_bound(&rkyv_path, field)?);

                            if let Some(remote_clause) =
                                serialize_remote_bound(&rkyv_path, field)?
                            {
                                serialize_where.predicates.push(remote_clause);
                            }
                        }
                    }
                    Fields::Unit => (),
                }
            }

            let serialize_arms = generate_serialize_arms(
                data,
                &rkyv_path,
                &resolver,
                &parse_quote!(Self),
                serialize
            )?;

            let serialize_impl = quote! {
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
                        Ok(match self {
                            #(#serialize_arms,)*
                        })
                    }
                }
            };

            let serialize_with_impl = if let Some(ref remote) = attributes.remote {
                let serialize_arms = generate_serialize_arms(
                    data,
                    &rkyv_path,
                    &resolver,
                    &strip_generics_from_path(remote.clone()),
                    serialize_remote
                )?;

                quote! {
                    #[automatically_derived]
                    impl #impl_generics #rkyv_path::with::SerializeWith<#remote, __S>
                        for #name #ty_generics
                    #serialize_where
                    {
                        fn serialize_with(
                            field: &#remote,
                            serializer: &mut __S,
                        ) -> ::core::result::Result<
                            <Self as #rkyv_path::Archive>::Resolver,
                            <__S as #rkyv_path::rancor::Fallible>::Error,
                        > {
                            ::core::result::Result::Ok(match field {
                                #(#serialize_arms,)*
                            })
                        }
                    }
                }
            } else {
                TokenStream::new()
            };

            (serialize_impl, serialize_with_impl)
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Serialize cannot be derived for unions",
            ))
        }
    };

    Ok(quote! {
        #[automatically_derived]
        #serialize_impl

        #serialize_with_impl
    })
}

fn generate_serialize_arms(
    data: &DataEnum,
    rkyv_path: &Path,
    resolver: &Ident,
    name: &Path,
    serialize_fn: fn(&Path, &Field) -> Result<TokenStream, Error>,
) -> Result<Vec<TokenStream>, Error> {
    data
        .variants
        .iter()
        .map(|v| {
            let variant = &v.ident;
            match v.fields {
                Fields::Named(ref fields) => {
                    let bindings =
                        fields.named.iter().map(|f| &f.ident);
                    let fields = fields
                        .named
                        .iter()
                        .map(|field| {
                            let name = &field.ident;
                            let serialize =
                                serialize_fn(rkyv_path, field)?;
                            Ok(quote! {
                                #name: #serialize(#name, serializer)?
                            })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;
                    Ok(quote! {
                        #name::#variant {
                            #(#bindings,)*
                        } => #resolver::#variant {
                            #(#fields,)*
                        }
                    })
                }
                Fields::Unnamed(ref fields) => {
                    let bindings =
                        fields.unnamed.iter().enumerate().map(
                            |(i, f)| {
                                Ident::new(&format!("_{}", i), f.span())
                            },
                        );

                    let fields = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, field)| {
                            let binding = Ident::new(
                                &format!("_{}", i),
                                field.span(),
                            );
                            let serialize =
                                serialize_fn(rkyv_path, field)?;
                            Ok(quote! {
                                #serialize(#binding, serializer)?
                            })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;
                    Ok(quote! {
                        #name::#variant(
                            #(#bindings,)*
                        ) => #resolver::#variant(#(#fields,)*)
                    })
                }
                Fields::Unit => {
                    Ok(quote! { #name::#variant => #resolver::#variant })
                }
            }
        })
        .collect()
}