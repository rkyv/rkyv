extern crate proc_macro;

use proc_macro2::{
    Span,
    TokenStream,
};
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    AttrStyle,
    Data,
    DeriveInput,
    Error,
    Fields,
    Ident,
    Index,
    Meta,
    MetaList,
    NestedMeta,
    parse_macro_input,
    spanned::Spanned,
};

struct Repr {
    rust: Option<Span>,
    transparent: Option<Span>,
    packed: Option<Span>,
    c: Option<Span>,
    int: Option<Span>,
}

impl Default for Repr {
    fn default() -> Self {
        Self {
            rust: None,
            transparent: None,
            packed: None,
            c: None,
            int: None,
        }
    }
}

struct Attributes {
    archive_self: Option<Span>,
    repr: Repr,
    derives: Option<MetaList>,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            archive_self: None,
            repr: Default::default(),
            derives: None,
        }
    }
}

fn parse_attributes(input: &DeriveInput) -> Result<Attributes, TokenStream> {
    let mut result = Attributes::default();
    for a in input.attrs.iter() {
        match a.style {
            AttrStyle::Outer => match a.parse_meta() {
                Ok(meta) => match meta {
                    Meta::List(meta) => if meta.path.is_ident("archive") {
                        for n in meta.nested.iter() {
                            match n {
                                NestedMeta::Meta(meta) => match meta {
                                    Meta::Path(path) => {
                                        if path.is_ident("self") {
                                            result.archive_self = Some(path.span());
                                        } else {
                                            return Err(Error::new(path.span(), "unrecognized archive attribute").to_compile_error());
                                        }
                                    },
                                    Meta::List(meta) => if meta.path.is_ident("derive") {
                                        result.derives = Some(meta.clone());
                                    },
                                    _ => (),
                                },
                                _ => (),
                            }
                        }
                    } else if meta.path.is_ident("repr") {
                        for n in meta.nested.iter() {
                            match n {
                                NestedMeta::Meta(meta) => match meta {
                                    Meta::Path(path) => {
                                        if path.is_ident("rust") {
                                            result.repr.rust = Some(path.span());
                                        } else if path.is_ident("transparent") {
                                            result.repr.transparent = Some(path.span());
                                        } else if path.is_ident("packed") {
                                            result.repr.packed = Some(path.span());
                                        } else if path.is_ident("C") {
                                            result.repr.c = Some(path.span());
                                        } else {
                                            result.repr.int = Some(path.span());
                                        }
                                    },
                                    _ => (),
                                },
                                _ => (),
                            }
                        }
                    },
                    _ => ()
                },
                _ => (),
            },
            _ => (),
        }
    }
    Ok(result)
}

#[proc_macro_derive(Archive, attributes(archive))]
pub fn archive_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let attributes = match parse_attributes(&input) {
        Ok(attributes) => attributes,
        Err(errors) => return proc_macro::TokenStream::from(errors),
    };

    let archive_impl = if attributes.archive_self.is_some() {
        derive_archive_self_impl(&input, &attributes)
    } else {
        derive_archive_impl(&input, &attributes)
    };

    proc_macro::TokenStream::from(archive_impl)
}

fn derive_archive_impl(input: &DeriveInput, attributes: &Attributes) -> TokenStream {
    let name = &input.ident;

    let generic_params = input.generics.params.iter().map(|p| quote! { #p });
    let generic_params = quote! { #(#generic_params,)* };

    let generic_args = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { p.ident.span() => #name }
    });
    let generic_args = quote! { #(#generic_args,)* };

    let generic_predicates = match input.generics.where_clause {
        Some(ref clause) => {
            let predicates = clause.predicates.iter().map(|p| quote! { #p });
            quote! { #(#predicates,)* }
        },
        None => quote! {},
    };

    let archive_derives = if let Some(derives) = attributes.derives.as_ref() {
        quote! { #[#derives] }
    } else {
        quote! {}
    };

    let archive_impl = match input.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: Archive }
                    });
                    let field_wheres = quote! { #(#field_wheres,)* };

                    let resolver_fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #name: archive::Resolver<#ty> }
                    });

                    let resolver_values = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! { f.span() => #name: self.#name.archive(writer)? }
                    });

                    let archived_fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #name: archive::Archived<#ty> }
                    });

                    let archived_values = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! { f.span() => #name: self.#name.resolve(pos + offset_of!(Archived<#generic_args>, #name), &value.#name) }
                    });

                    quote! {
                        struct Resolver<#generic_params>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            #(#resolver_fields,)*
                        }

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;

                            fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                                Self::Archived {
                                    #(#archived_values,)*
                                }
                            }
                        }

                        #archive_derives
                        struct Archived<#generic_params>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            #(#archived_fields,)*
                        }

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;
                            type Resolver = Resolver<#generic_args>;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Self::Resolver {
                                    #(#resolver_values,)*
                                })
                            }
                        }
                    }
                },
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: Archive }
                    });
                    let field_wheres = quote! { #(#field_wheres,)* };

                    let resolver_fields = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => archive::Resolver<#ty> }
                    });

                    let resolver_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let index = Index::from(i);
                        quote_spanned! { f.span() => self.#index.archive(writer)? }
                    });

                    let archived_fields = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => archive::Archived<#ty> }
                    });

                    let archived_values = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let index = Index::from(i);
                        quote_spanned! { f.span() => self.#index.resolve(pos + offset_of!(Archived<#generic_args>, #index), &value.#index) }
                    });

                    quote! {
                        struct Resolver<#generic_params>(#(#resolver_fields,)*)
                        where
                            #generic_predicates
                            #field_wheres;

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;

                            fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                                Archived::<#generic_args>(
                                    #(#archived_values,)*
                                )
                            }
                        }

                        #archive_derives
                        struct Archived<#generic_params>(#(#archived_fields,)*)
                        where
                            #generic_predicates
                            #field_wheres;

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                            #field_wheres
                        {
                            type Archived = Archived<#generic_args>;
                            type Resolver = Resolver<#generic_args>;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Resolver::<#generic_args>(
                                    #(#resolver_values,)*
                                ))
                            }
                        }
                    }
                },
                Fields::Unit => {
                    quote! {
                        struct Resolver;

                        impl<#generic_params> Resolve<#name<#generic_args>> for Resolver
                        where
                            #generic_predicates
                        {
                            type Archived = Archived<#generic_args>;

                            fn resolve(self, _pos: usize, _value: &#name<#generic_args>) -> Self::Archived {
                                Archived::<#generic_args>
                            }
                        }

                        #archive_derives
                        struct Archived<#generic_params>
                        where
                            #generic_predicates;

                        impl<#generic_params> Archive for #name<#generic_args>
                        where
                            #generic_predicates
                        {
                            type Archived = Archived;
                            type Resolver = Resolver;

                            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                                Ok(Resolver)
                            }
                        }
                    }
                }
            }
        },
        Data::Enum(ref data) => {
            let field_wheres = data.variants.iter().map(|v| {
                match v.fields {
                    Fields::Named(ref fields) => {
                        let field_wheres = fields.named.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() =>  #ty: Archive }
                        });
                        quote! { #(#field_wheres,)* }
                    },
                    Fields::Unnamed(ref fields) => {
                        let field_wheres = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #ty: Archive }
                        });
                        quote! { #(#field_wheres,)* }
                    },
                    Fields::Unit => quote! {},
                }
            });
            let field_wheres = quote! { #(#field_wheres)* };

            let resolver_variants = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #name: archive::Resolver<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant {
                                #(#fields,)*
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => archive::Resolver<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant(#(#fields,)*)
                        }
                    },
                    Fields::Unit => quote_spanned! { variant.span() => #variant },
                }
            });

            let resolve_arms = data.variants.iter().map(|v| {
                let variant = &v.ident;
                let archived_variant_name = Ident::new(&format!("ArchivedVariant{}", variant.to_string()), v.span());
                match v.fields {
                    Fields::Named(ref fields) => {
                        let self_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("self_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote_spanned! { name.span() => #name: #binding }
                        });
                        let value_bindings = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let binding = Ident::new(&format!("value_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote_spanned! { binding.span() => #name: #binding }
                        });
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let self_binding = Ident::new(&format!("self_{}", name.as_ref().unwrap().to_string()), name.span());
                            let value_name = Ident::new(&format!("value_{}", name.as_ref().unwrap().to_string()), name.span());
                            quote! {
                                #name: #self_binding.resolve(pos + offset_of!(#archived_variant_name<#generic_args>, #name), #value_name)
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant { #(#self_bindings,)* } => {
                                if let #name::#variant { #(#value_bindings,)* } = value { Archived::#variant { #(#fields,)* } } else { panic!("enum resolver variant does not match value variant") }
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let self_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("self_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let value_bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("value_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let index = Index::from(i + 1);
                            let self_binding = Ident::new(&format!("self_{}", i), f.span());
                            let value_binding = Ident::new(&format!("value_{}", i), f.span());
                            quote! {
                                #self_binding.resolve(pos + offset_of!(#archived_variant_name<#generic_args>, #index), #value_binding)
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant( #(#self_bindings,)* ) => {
                                if let #name::#variant(#(#value_bindings,)*) = value { Archived::#variant(#(#fields,)*) } else { panic!("enum resolver variant does not match value variant") }
                            }
                        }
                    },
                    Fields::Unit => quote_spanned! { name.span() => Self::#variant => Archived::#variant },
                }
            });

            let archived_repr = match data.variants.len() {
                0..=255 => quote! { u8 },
                256..=65_535 => quote! { u16 },
                65_536..=4_294_967_295 => quote! { u32 },
                4_294_967_296..=18_446_744_073_709_551_615 => quote! { u64 },
                _ => quote! { u128 },
            };

            let archived_variants = data.variants.iter().map(|v| {
                let variant = &v.ident;
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #name: archive::Archived<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant {
                                #(#fields,)*
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => archive::Archived<#ty> }
                        });
                        quote_spanned! { variant.span() =>
                            #variant(#(#fields,)*)
                        }
                    },
                    Fields::Unit => quote_spanned! { variant.span() => #variant },
                }
            });

            let archived_variant_tags = data.variants.iter().map(|v| {
                let variant = &v.ident;
                quote_spanned! { variant.span() => #variant }
            });

            let archived_variant_structs = data.variants.iter().map(|v| {
                let variant = &v.ident;
                let archived_variant_name = Ident::new(&format!("ArchivedVariant{}", variant.to_string()), v.span());
                match v.fields {
                    Fields::Named(ref fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = &f.ident;
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #name: archive::Archived<#ty> }
                        });
                        quote_spanned! { name.span() =>
                            #[repr(C)]
                            struct #archived_variant_name<#generic_params>
                            where
                                #generic_predicates
                                #field_wheres
                            {
                                __tag: ArchivedTag,
                                #(#fields,)*
                                __phantom: PhantomData<(#generic_args)>,
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let fields = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => archive::Archived<#ty> }
                        });
                        quote_spanned! { name.span() =>
                            #[repr(C)]
                            struct #archived_variant_name<#generic_params>(ArchivedTag, #(#fields,)* PhantomData<(#generic_args)>)
                            where
                                #generic_predicates
                                #field_wheres;
                        }
                    },
                    Fields::Unit => quote! {},
                }
            });

            let archive_arms = data.variants.iter().map(|v| {
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
                                #name: #name.archive(writer)?
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant { #(#bindings,)* } => Resolver::#variant {
                                #(#fields,)*
                            }
                        }
                    },
                    Fields::Unnamed(ref fields) => {
                        let bindings = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let name = Ident::new(&format!("_{}", i), f.span());
                            quote_spanned! { f.span() => #name }
                        });
                        let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            let binding = Ident::new(&format!("_{}", i), f.span());
                            quote! {
                                #binding.archive(writer)?
                            }
                        });
                        quote_spanned! { name.span() =>
                            Self::#variant( #(#bindings,)* ) => Resolver::#variant(#(#fields,)*)
                        }
                    },
                    Fields::Unit => quote_spanned! { name.span() => Self::#variant => Resolver::#variant },
                }
            });

            quote! {
                enum Resolver<#generic_params>
                where
                    #generic_predicates
                    #field_wheres
                {
                    #(#resolver_variants,)*
                }

                impl<#generic_params> Resolve<#name<#generic_args>> for Resolver<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Archived<#generic_args>;

                    fn resolve(self, pos: usize, value: &#name<#generic_args>) -> Self::Archived {
                        match self {
                            #(#resolve_arms,)*
                        }
                    }
                }

                #archive_derives
                #[repr(#archived_repr)]
                enum Archived<#generic_params>
                where
                    #generic_predicates
                    #field_wheres
                {
                    #(#archived_variants,)*
                }

                #[repr(#archived_repr)]
                enum ArchivedTag {
                    #(#archived_variant_tags,)*
                }

                #(#archived_variant_structs)*

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Archived<#generic_args>;
                    type Resolver = Resolver<#generic_args>;

                    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                        Ok(match self {
                            #(#archive_arms,)*
                        })
                    }
                }
            }
        },
        Data::Union(_) => Error::new(input.span(), "Archive cannot be derived for unions").to_compile_error(),
    };

    quote! {
        const _: () = {
            use core::marker::PhantomData;
            use archive::{
                Archive,
                offset_of,
                Resolve,
                Write,
            };
            #archive_impl
        };
    }
}

fn derive_archive_self_impl(input: &DeriveInput, attributes: &Attributes) -> TokenStream {
    let name = &input.ident;

    let generic_params = input.generics.params.iter().map(|p| quote! { #p });
    let generic_params = quote! { #(#generic_params,)* };

    let generic_args = input.generics.type_params().map(|p| {
        let name = &p.ident;
        quote_spanned! { p.ident.span() => #name }
    });
    let generic_args = quote! { #(#generic_args,)* };

    let generic_predicates = match input.generics.where_clause {
        Some(ref clause) => {
            let predicates = clause.predicates.iter().map(|p| quote! { #p });
            quote! { #(#predicates,)* }
        },
        None => quote! {},
    };

    let archive_self_impl = match input.data {
        Data::Struct(ref data) => {
            let field_wheres = match data.fields {
                Fields::Named(ref fields) => {
                    let field_wheres = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveSelf }
                    });

                    quote! { #(#field_wheres,)* }
                },
                Fields::Unnamed(ref fields) => {
                    let field_wheres = fields.unnamed.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! { f.span() => #ty: ArchiveSelf }
                    });

                    quote! { #(#field_wheres,)* }
                },
                Fields::Unit => quote! {},
            };

            quote! {
                unsafe impl<#generic_params> ArchiveSelf for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {}

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Self;
                    type Resolver = SelfResolver;

                    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                        Ok(SelfResolver)
                    }
                }
            }
        },
        Data::Enum(ref data) => {
            if let Some(span) = attributes.repr.rust.or(attributes.repr.transparent).or(attributes.repr.packed) {
                return Error::new(span, "archive self enums must be repr(C) or repr(Int)").to_compile_error();
            }

            if attributes.repr.c.is_none() && attributes.repr.int.is_none() {
                return Error::new(input.span(), "archive self enums must be repr(C) or repr(Int)").to_compile_error();
            }

            let field_wheres = data.variants.iter().map(|v| {
                match v.fields {
                    Fields::Named(ref fields) => {
                        let field_wheres = fields.named.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #ty: ArchiveSelf }
                        });
                        quote! { #(#field_wheres,)* }
                    },
                    Fields::Unnamed(ref fields) => {
                        let field_wheres = fields.unnamed.iter().map(|f| {
                            let ty = &f.ty;
                            quote_spanned! { f.span() => #ty: ArchiveSelf }
                        });
                        quote! { #(#field_wheres,)* }
                    },
                    Fields::Unit => quote! {},
                }
            });
            let field_wheres = quote! { #(#field_wheres)* };

            quote! {
                unsafe impl<#generic_params> ArchiveSelf for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {}

                impl<#generic_params> Archive for #name<#generic_args>
                where
                    #generic_predicates
                    #field_wheres
                {
                    type Archived = Self;
                    type Resolver = SelfResolver;

                    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                        Ok(SelfResolver)
                    }
                }
            }
        },
        Data::Union(_) => Error::new(input.span(), "Archive cannot be derived for unions").to_compile_error(),
    };

    quote! {
        const _: () = {
            use archive::{
                Archive,
                ArchiveSelf,
                SelfResolver,
                Write,
            };

            #archive_self_impl
        };
    }
}
