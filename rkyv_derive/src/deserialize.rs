use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput,
    Error, Fields, Generics, Ident, Index,
};

use crate::{
    attributes::Attributes,
    util::{
        archive_bound, archive_remote_bound, deserialize, deserialize_bound,
        is_not_omitted,
    },
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

    let (deserialize_impl, deserialize_with_impl) = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let mut deserialize_where = where_clause.clone();
                for field in fields.named.iter().filter(is_not_omitted) {
                    deserialize_where
                        .predicates
                        .push(archive_bound(&rkyv_path, field)?);
                    deserialize_where
                        .predicates
                        .push(deserialize_bound(&rkyv_path, field)?);

                    if let Some(remote_clause) =
                        archive_remote_bound(&rkyv_path, field)?
                    {
                        deserialize_where.predicates.push(remote_clause);
                    }
                }

                let deserialize_fields = fields
                    .named
                    .iter()
                    .map(|field| {
                        let name = &field.ident;
                        let deserialize = deserialize(&rkyv_path, field)?;
                        Ok(quote! {
                            #name: #deserialize(&self.#name, deserializer)?
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                let deserialize_impl = quote! {
                    impl #impl_generics
                        #rkyv_path::Deserialize<#name #ty_generics, __D>
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
                            ::core::result::Result::Ok(#name {
                                #(#deserialize_fields,)*
                            })
                        }
                    }
                };

                let deserialize_with_impl = if let Some(ref remote) =
                    attributes.remote
                {
                    quote! {
                        #[automatically_derived]
                        impl #impl_generics
                            #rkyv_path::with::DeserializeWith<
                                #rkyv_path::Archived<#name #ty_generics>,
                                #remote,
                                __D,
                            >
                            for #name #ty_generics
                        #deserialize_where
                        {
                            fn deserialize_with(
                                field: &#rkyv_path::Archived<
                                    #name #ty_generics
                                >,
                                deserializer: &mut __D,
                            ) -> ::core::result::Result<
                                #remote,
                                <__D as #rkyv_path::rancor::Fallible>::Error,
                            > {
                                field.deserialize(deserializer).map(From::from)
                            }
                        }
                    }
                } else {
                    TokenStream::new()
                };

                (deserialize_impl, deserialize_with_impl)
            }
            Fields::Unnamed(ref fields) => {
                let mut deserialize_where = where_clause.clone();
                for field in fields.unnamed.iter().filter(is_not_omitted) {
                    deserialize_where
                        .predicates
                        .push(archive_bound(&rkyv_path, field)?);
                    deserialize_where
                        .predicates
                        .push(deserialize_bound(&rkyv_path, field)?);

                    if let Some(remote_clause) =
                        archive_remote_bound(&rkyv_path, field)?
                    {
                        deserialize_where.predicates.push(remote_clause);
                    }
                }

                let deserialize_fields = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, field)| {
                        let index = Index::from(i);
                        let deserialize = deserialize(&rkyv_path, field)?;
                        Ok(quote! {
                            #deserialize(
                                &self.#index,
                                deserializer,
                            )?
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                let deserialize_impl = quote! {
                    impl #impl_generics
                        #rkyv_path::Deserialize<#name #ty_generics, __D>
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
                            ::core::result::Result::Ok(#name(
                                #(#deserialize_fields,)*
                            ))
                        }
                    }
                };

                let deserialize_with_impl =
                    if let Some(ref remote) = attributes.remote {
                        quote! {
                            #[automatically_derived]
                            impl #impl_generics
                                #rkyv_path::with::DeserializeWith<
                                    #rkyv_path::Archived<#name #ty_generics>,
                                    #remote,
                                    __D,
                                >
                                for #name #ty_generics
                            #deserialize_where
                            {
                                fn deserialize_with(
                                    field: &#rkyv_path::Archived<
                                        #name #ty_generics
                                    >,
                                    deserializer: &mut __D,
                                ) -> ::core::result::Result<
                                    #remote,
                                    <
                                        __D as #rkyv_path::rancor::Fallible
                                    >::Error,
                                > {
                                    field.deserialize(deserializer)
                                        .map(From::from)
                                }
                            }
                        }
                    } else {
                        TokenStream::new()
                    };

                (deserialize_impl, deserialize_with_impl)
            }
            Fields::Unit => {
                let deserialize_impl = quote! {
                    impl #impl_generics
                        #rkyv_path::Deserialize<#name #ty_generics, __D>
                        for #rkyv_path::Archived<#name #ty_generics>
                    #where_clause
                    {
                        fn deserialize(
                            &self,
                            _: &mut __D,
                        ) -> ::core::result::Result<
                            #name #ty_generics,
                            <__D as #rkyv_path::rancor::Fallible>::Error,
                        > {
                            ::core::result::Result::Ok(#name)
                        }
                    }
                };

                let deserialize_with_impl =
                    if let Some(ref remote) = attributes.remote {
                        quote! {
                            #[automatically_derived]
                            impl #impl_generics
                                #rkyv_path::with::DeserializeWith<
                                    #rkyv_path::Archived<#name #ty_generics>,
                                    #remote,
                                    __D,
                                >
                                for #name #ty_generics
                            #where_clause
                            {
                                fn deserialize_with(
                                    field: &#rkyv_path::Archived<
                                        #name #ty_generics
                                    >,
                                    _: &mut __D,
                                ) -> ::core::result::Result<
                                    #remote,
                                    <
                                        __D as #rkyv_path::rancor::Fallible
                                    >::Error,
                                > {
                                    Ok(#remote)
                                }
                            }
                        }
                    } else {
                        TokenStream::new()
                    };

                (deserialize_impl, deserialize_with_impl)
            }
        },
        Data::Enum(ref data) => {
            let mut deserialize_where = where_clause.clone();
            for variant in data.variants.iter() {
                match variant.fields {
                    Fields::Named(ref fields) => {
                        for field in fields.named.iter().filter(is_not_omitted)
                        {
                            deserialize_where
                                .predicates
                                .push(archive_bound(&rkyv_path, field)?);
                            deserialize_where
                                .predicates
                                .push(deserialize_bound(&rkyv_path, field)?);

                            if let Some(remote_clause) =
                                archive_remote_bound(&rkyv_path, field)?
                            {
                                deserialize_where
                                    .predicates
                                    .push(remote_clause);
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        for field in
                            fields.unnamed.iter().filter(is_not_omitted)
                        {
                            deserialize_where
                                .predicates
                                .push(archive_bound(&rkyv_path, field)?);
                            deserialize_where
                                .predicates
                                .push(deserialize_bound(&rkyv_path, field)?);

                            if let Some(remote_clause) =
                                archive_remote_bound(&rkyv_path, field)?
                            {
                                deserialize_where
                                    .predicates
                                    .push(remote_clause);
                            }
                        }
                    }
                    Fields::Unit => (),
                }
            }

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
                                    let name = &field.ident;
                                    let deserialize =
                                        deserialize(&rkyv_path, field)?;
                                    Ok(quote! {
                                        #name: #deserialize(
                                            #name,
                                            deserializer,
                                        )?
                                    })
                                })
                                .collect::<Result<Vec<_>, Error>>()?;
                            Ok(quote! {
                                Self::#variant {
                                    #(#bindings,)*
                                } => #name::#variant { #(#fields,)* }
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
                                    let deserialize =
                                        deserialize(&rkyv_path, field)?;
                                    Ok(quote! {
                                        #deserialize(
                                            #binding,
                                            deserializer,
                                        )?
                                    })
                                })
                                .collect::<Result<Vec<_>, Error>>()?;
                            Ok(quote! {
                                Self::#variant(
                                    #(#bindings,)*
                                ) => #name::#variant(#(#fields,)*)
                            })
                        }
                        Fields::Unit => {
                            Ok(quote! { Self::#variant => #name::#variant })
                        }
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?;

            let deserialize_impl = quote! {
                impl #impl_generics
                    #rkyv_path::Deserialize<#name #ty_generics, __D>
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
                        ::core::result::Result::Ok(match self {
                            #(#deserialize_variants,)*
                        })
                    }
                }
            };

            let deserialize_with_impl =
                if let Some(ref remote) = attributes.remote {
                    quote! {
                        #[automatically_derived]
                        impl #impl_generics
                            #rkyv_path::with::DeserializeWith<
                                #rkyv_path::Archived<#name #ty_generics>,
                                #remote,
                                __D,
                            >
                            for #name #ty_generics
                        #deserialize_where
                        {
                            fn deserialize_with(
                                field: &#rkyv_path::Archived<
                                    #name #ty_generics
                                >,
                                deserializer: &mut __D,
                            ) -> ::core::result::Result<
                                #remote,
                                <__D as #rkyv_path::rancor::Fallible>::Error,
                            > {
                                field.deserialize(deserializer).map(From::from)
                            }
                        }
                    }
                } else {
                    TokenStream::new()
                };

            (deserialize_impl, deserialize_with_impl)
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "Deserialize cannot be derived for unions",
            ))
        }
    };

    Ok(quote! {
        #[automatically_derived]
        #deserialize_impl

        #deserialize_with_impl
    })
}
