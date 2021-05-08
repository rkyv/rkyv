use crate::attributes::{parse_attributes, Attributes};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput, Error, Fields, Ident,
    Index, Token, WherePredicate, Generics,
};

pub fn derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = parse_attributes(&input)?;
    derive_deserialize_impl(input, &attributes)
}

fn derive_deserialize_impl(
    mut input: DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    let where_clause = input.generics.make_where_clause();
    if let Some(ref bounds) = attributes.deserialize_bound {
        let clauses =
            bounds.parse_with(Punctuated::<WherePredicate, Token![,]>::parse_terminated)?;
        for clause in clauses {
            where_clause.predicates.push(clause);
        }
    }

    let mut impl_input_params = Punctuated::default();
    impl_input_params.push(parse_quote! { __D: Fallible + ?Sized });
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
    let (impl_generics, _, _) = impl_input_generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let where_clause = where_clause.unwrap();

    let deserialize_impl = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let mut deserialize_where = where_clause.clone();
                for field in fields
                    .named
                    .iter()
                    .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                {
                    let ty = &field.ty;
                    deserialize_where
                        .predicates
                        .push(parse_quote! { #ty: Archive });
                    deserialize_where
                        .predicates
                        .push(parse_quote! { Archived<#ty>: Deserialize<#ty, __D> });
                }

                let deserialize_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote! { #name: self.#name.deserialize(deserializer)? }
                });

                quote! {
                    impl #impl_generics Deserialize<#name #ty_generics, __D> for Archived<#name #ty_generics> #deserialize_where {
                        #[inline]
                        fn deserialize(&self, deserializer: &mut __D) -> Result<#name #ty_generics, __D::Error> {
                            Ok(#name {
                                #(#deserialize_fields,)*
                            })
                        }
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let mut deserialize_where = where_clause.clone();
                for field in fields
                    .unnamed
                    .iter()
                    .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                {
                    let ty = &field.ty;
                    deserialize_where
                        .predicates
                        .push(parse_quote! { #ty: Archive });
                    deserialize_where
                        .predicates
                        .push(parse_quote! { Archived<#ty>: Deserialize<#ty, __D> });
                }

                let deserialize_fields = fields.unnamed.iter().enumerate().map(|(i, _)| {
                    let index = Index::from(i);
                    quote! { self.#index.deserialize(deserializer)? }
                });

                quote! {
                    impl #impl_generics Deserialize<#name #ty_generics, __D> for Archived<#name #ty_generics> #deserialize_where {
                        #[inline]
                        fn deserialize(&self, deserializer: &mut __D) -> Result<#name #ty_generics, __D::Error> {
                            Ok(#name(
                                #(#deserialize_fields,)*
                            ))
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl #impl_generics Deserialize<#name #ty_generics, __D> for Archived<#name #ty_generics> #where_clause {
                    #[inline]
                    fn deserialize(&self, _: &mut __D) -> Result<#name #ty_generics, __D::Error> {
                        Ok(#name)
                    }
                }
            },
        },
        Data::Enum(ref data) => {
            let mut deserialize_where = where_clause.clone();
            for variant in data.variants.iter() {
                match variant.fields {
                    Fields::Named(ref fields) => {
                        for field in fields
                            .named
                            .iter()
                            .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                        {
                            let ty = &field.ty;
                            deserialize_where
                                .predicates
                                .push(parse_quote! { #ty: Archive });
                            deserialize_where
                                .predicates
                                .push(parse_quote! { Archived<#ty>: Deserialize<#ty, __D> });
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        for field in fields
                            .unnamed
                            .iter()
                            .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                        {
                            let ty = &field.ty;
                            deserialize_where
                                .predicates
                                .push(parse_quote! { #ty: Archive });
                            deserialize_where
                                .predicates
                                .push(parse_quote! { Archived<#ty>: Deserialize<#ty, __D> });
                        }
                    }
                    Fields::Unit => (),
                }
            }

            let deserialize_variants = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            quote_spanned! { name.span() => #name }
                        });
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            quote! {
                                #name: #name.deserialize(deserializer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant { #(#bindings,)* } => #name::#variant { #(#fields,)* }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("_{}", i), f.span());
                            quote_spanned! { name.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let binding = Ident::new(&format!("_{}", i), f.span());
                            quote! {
                                #binding.deserialize(deserializer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant( #(#bindings,)* ) => #name::#variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => {
                        quote_spanned! { name.span() => Self::#variant => #name::#variant }
                    }
                }
            });

            quote! {
                impl #impl_generics Deserialize<#name #ty_generics, __D> for Archived<#name #ty_generics> #deserialize_where {
                    #[inline]
                    fn deserialize(&self, deserializer: &mut __D) -> Result<#name #ty_generics, __D::Error> {
                        Ok(match self {
                            #(#deserialize_variants,)*
                        })
                    }
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

    Ok(quote! {
        const _: () = {
            use ::rkyv::{Archive, Archived, Deserialize, Fallible};
            #deserialize_impl
        };
    })
}
