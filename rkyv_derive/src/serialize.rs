use crate::attributes::{parse_attributes, Attributes};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput, Error, Fields, Ident,
    Index, Token, WherePredicate, Generics,
};

pub fn derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let attributes = parse_attributes(&input)?;

    if attributes.copy.is_some() {
        derive_serialize_copy_impl(input, &attributes)
    } else {
        derive_serialize_impl(input, &attributes)
    }
}

fn derive_serialize_impl(
    mut input: DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    let where_clause = input.generics.make_where_clause();
    if let Some(ref bounds) = attributes.serialize_bound {
        let clauses = bounds.parse_with(Punctuated::<WherePredicate, Token![,]>::parse_terminated)?;
        for clause in clauses {
            where_clause.predicates.push(clause);
        }
    }

    let mut impl_input_params = Punctuated::default();
    impl_input_params.push(parse_quote! { __S: Fallible + ?Sized });
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

    let resolver = attributes.resolver.as_ref().map_or_else(
        || Ident::new(&format!("{}Resolver", name), name.span()),
        |value| value.clone(),
    );

    let serialize_impl = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let mut serialize_where = where_clause.clone();
                for field in fields
                    .named
                    .iter()
                    .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                {
                    let ty = &field.ty;
                    serialize_where
                        .predicates
                        .push(parse_quote! { #ty: Serialize<__S> });
                }

                let resolver_values = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! { f.span() => #name: Serialize::<__S>::serialize(&self.#name, serializer)? }
                });

                quote! {
                    impl #impl_generics Serialize<__S> for #name #ty_generics #serialize_where {
                        #[inline]
                        fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                            Ok(#resolver {
                                #(#resolver_values,)*
                            })
                        }
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let mut serialize_where = where_clause.clone();
                for field in fields
                    .unnamed
                    .iter()
                    .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                {
                    let ty = &field.ty;
                    serialize_where
                        .predicates
                        .push(parse_quote! { #ty: Serialize<__S> });
                }

                let resolver_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! { f.span() => Serialize::<__S>::serialize(&self.#index, serializer)? }
                });

                quote! {
                    impl #impl_generics Serialize<__S> for #name #ty_generics #serialize_where {
                        #[inline]
                        fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                            Ok(#resolver(
                                #(#resolver_values,)*
                            ))
                        }
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    impl #impl_generics Serialize<__S> for #name #ty_generics #where_clause {
                        #[inline]
                        fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                            Ok(#resolver)
                        }
                    }
                }
            }
        },
        Data::Enum(ref data) => {
            let mut serialize_where = where_clause.clone();
            for variant in data.variants.iter() {
                match variant.fields {
                    Fields::Named(ref fields) => {
                        for field in fields
                            .named
                            .iter()
                            .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                        {
                            let ty = &field.ty;
                            serialize_where
                                .predicates
                                .push(parse_quote! { #ty: Serialize<__S> });
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        for field in fields
                            .unnamed
                            .iter()
                            .filter(|f| !f.attrs.iter().any(|a| a.path.is_ident("omit_bounds")))
                        {
                            let ty = &field.ty;
                            serialize_where
                                .predicates
                                .push(parse_quote! { #ty: Serialize<__S> });
                        }
                    }
                    Fields::Unit => (),
                }
            }

            let serialize_arms = data.variants.iter().map(|v| {
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
                                #name: Serialize::<__S>::serialize(#name, serializer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant { #(#bindings,)* } => #resolver::#variant {
                                #(#fields,)*
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let binding = Ident::new(&format!("_{}", i), f.span());
                            quote! {
                                Serialize::<__S>::serialize(#binding, serializer)?
                            }
                        });
                        quote_spanned! { variant.span() =>
                            Self::#variant( #(#bindings,)* ) => #resolver::#variant(#(#fields,)*)
                        }
                    }
                    Fields::Unit => {
                        quote_spanned! { name.span() => Self::#variant => #resolver::#variant }
                    }
                }
            });

            quote! {
                impl #impl_generics Serialize<__S> for #name #ty_generics #serialize_where {
                    #[inline]
                    fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                        Ok(match self {
                            #(#serialize_arms,)*
                        })
                    }
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

    Ok(quote! {
        const _: () = {
            use rkyv::{Archive, Fallible, Serialize};
            #serialize_impl
        };
    })
}

fn derive_serialize_copy_impl(
    mut input: DeriveInput,
    attributes: &Attributes,
) -> Result<TokenStream, Error> {
    if let Some(ref archived) = attributes.archived {
        return Err(Error::new_spanned(
            archived,
            "archive copy types cannot be named",
        ));
    } else if let Some(ref resolver) = attributes.resolver {
        return Err(Error::new_spanned(
            resolver,
            "archive copy resolvers cannot be named",
        ));
    };

    input.generics.make_where_clause();

    let mut impl_input_params = Punctuated::default();
    impl_input_params.push(parse_quote! { __S: Fallible + ?Sized });
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

    let serialize_copy_impl = match input.data {
        Data::Struct(ref data) => {
            let mut copy_where = where_clause.clone();
            match data.fields {
                Fields::Named(ref fields) => {
                    for field in fields.named.iter() {
                        let ty = &field.ty;
                        copy_where
                            .predicates
                            .push(parse_quote! { #ty: ArchiveCopy });
                    }
                }
                Fields::Unnamed(ref fields) => {
                    for field in fields.unnamed.iter() {
                        let ty = &field.ty;
                        copy_where
                            .predicates
                            .push(parse_quote! { #ty: ArchiveCopy });
                    }
                }
                Fields::Unit => (),
            };

            quote! {
                impl #impl_generics Serialize<__S> for #name #ty_generics #copy_where {
                    #[inline]
                    fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                        Ok(())
                    }
                }
            }
        }
        Data::Enum(ref data) => {
            if let Some(ref path) = attributes
                .repr
                .rust
                .as_ref()
                .or_else(|| attributes.repr.transparent.as_ref())
                .or_else(|| attributes.repr.packed.as_ref())
            {
                return Err(Error::new_spanned(
                    path,
                    "archive copy enums must be repr(C) or repr(Int)",
                ));
            }

            if attributes.repr.c.is_none() && attributes.repr.int.is_none() {
                return Err(Error::new_spanned(
                    input,
                    "archive copy enums must be repr(C) or repr(Int)",
                ));
            }

            let mut copy_where = where_clause.clone();
            for variant in data.variants.iter() {
                match variant.fields {
                    Fields::Named(ref fields) => {
                        for field in fields.named.iter() {
                            let ty = &field.ty;
                            copy_where
                                .predicates
                                .push(parse_quote! { #ty: ArchiveCopy });
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        for field in fields.unnamed.iter() {
                            let ty = &field.ty;
                            copy_where
                                .predicates
                                .push(parse_quote! { #ty: ArchiveCopy });
                        }
                    }
                    Fields::Unit => (),
                }
            }

            quote! {
                impl #impl_generics Serialize<__S> for #name #ty_generics #copy_where {
                    #[inline]
                    fn serialize(&self, serializer: &mut __S) -> Result<Self::Resolver, __S::Error> {
                        Ok(())
                    }
                }
            }
        }
        Data::Union(_) => {
            Error::new(input.span(), "Serialize cannot be derived for unions").to_compile_error()
        }
    };

    Ok(quote! {
        const _: () = {
            use rkyv::{Archive, ArchiveCopy, Fallible, Serialize};

            #serialize_copy_impl
        };
    })
}
