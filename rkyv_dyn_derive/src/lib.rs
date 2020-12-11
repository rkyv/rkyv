//! Procedural macros for `rkyv_dyn`.

extern crate proc_macro;

use quote::{quote, quote_spanned};
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    spanned::Spanned,
    Attribute, Error, Ident, ItemImpl, ItemTrait, LitStr, Token, Visibility,
};

enum Input {
    Impl(ItemImpl),
    Trait(ItemTrait),
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = Attribute::parse_outer(input)?;

        let ahead = input.fork();
        ahead.parse::<Visibility>()?;
        ahead.parse::<Option<Token![unsafe]>>()?;

        if ahead.peek(Token![trait]) {
            let mut item: ItemTrait = input.parse()?;
            attrs.extend(item.attrs);
            item.attrs = attrs;
            Ok(Input::Trait(item))
        } else if ahead.peek(Token![impl]) {
            let mut item: ItemImpl = input.parse()?;
            if item.trait_.is_none() {
                let impl_token = item.impl_token;
                let ty = item.self_ty;
                let span = quote!(#impl_token #ty);
                let msg = "expected impl Trait for Type";
                return Err(Error::new_spanned(span, msg));
            }
            attrs.extend(item.attrs);
            item.attrs = attrs;
            Ok(Input::Impl(item))
        } else {
            Err(input.error("expected trait or impl block"))
        }
    }
}

enum TraitArgs {
    None,
    Trait(LitStr),
}

impl Parse for TraitArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(TraitArgs::None);
        }
        input.parse::<syn::token::Trait>()?;
        input.parse::<Token![=]>()?;
        let name = input.parse::<LitStr>()?;
        Ok(TraitArgs::Trait(name))
    }
}

/// Creates archiveable trait objects and registers implementations.
///
/// Prepend to trait definitions and implementations. On trait definitions, you
/// can use the form `#[archive_dyn = "..."]` to choose the name of the archive
/// type. By default, it will be named "Archive" + your trait name.
#[proc_macro_attribute]
pub fn archive_dyn(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as Input);

    let input_impl = match input {
        Input::Impl(ref input) => {
            if !input.generics.params.is_empty() {
                Error::new(input.generics.span(), "#[archive_dyn] can only register non-generic impls; use register_vtable! with the concrete types to register").to_compile_error()
            } else if let Some((_, ref trait_, _)) = input.trait_ {
                let ty = &input.self_ty;
                quote! {
                    #input

                    rkyv_dyn::register_vtable!(#ty as dyn #trait_);
                }
            } else {
                Error::new(
                    input.span(),
                    "#[archive_dyn] is only valid on trait implementations",
                )
                .to_compile_error()
            }
        }
        Input::Trait(input) => {
            let args = parse_macro_input!(attr as TraitArgs);

            let vis = &input.vis;

            let generic_params = input
                .generics
                .params
                .iter()
                .map(|p| quote_spanned! { p.span() => #p });
            let generic_params = quote! { #(#generic_params),* };

            let generic_args = input.generics.type_params().map(|p| {
                let name = &p.ident;
                quote_spanned! { name.span() => #name }
            });
            let generic_args = quote! { #(#generic_args),* };

            let name = &input.ident;
            let archive_trait = match args {
                TraitArgs::None => Ident::new(&format!("Archive{}", name), name.span()),
                TraitArgs::Trait(name) => Ident::new(&name.value(), name.span()),
            };

            let type_name_wheres = input.generics.type_params().map(|p| {
                let name = &p.ident;
                quote_spanned! { name.span() => #name: TypeName }
            });
            let type_name_wheres = quote! { #(#type_name_wheres,)* };

            let build_type_name = if !input.generics.params.is_empty() {
                let dyn_name = format!("dyn {}<", name);
                let mut results = input.generics.type_params().map(|p| {
                    let name = &p.ident;
                    quote_spanned! { name.span() => #name::build_type_name(&mut f) }
                });
                let first = results.next().unwrap();
                quote! {
                    f(#dyn_name);
                    #first;
                    #(f(", "); #results;)*
                    f(">");
                }
            } else {
                quote! { f(stringify!(dyn #name)); }
            };

            quote! {
                #input

                #vis trait #archive_trait<#generic_params>: #name<#generic_args> + rkyv_dyn::ArchiveDyn {}

                const _: ()  = {
                    use rkyv::{
                        Archived,
                        ArchiveRef,
                        Resolve,
                        Write,
                    };
                    use rkyv_dyn::{
                        ArchiveDyn,
                        ArchivedDyn,
                        DynResolver,
                        hash_value,
                        TraitObject,
                    };
                    use rkyv_typename::TypeName;

                    impl<#generic_params> TypeName for dyn #name<#generic_args> + '_
                    where
                        #type_name_wheres
                    {
                        fn build_type_name<F: FnMut(&str)>(mut f: F) {
                            #build_type_name
                        }
                    }

                    impl<#generic_params> TypeName for dyn #archive_trait<#generic_args> + '_
                    where
                        #type_name_wheres
                    {
                        fn build_type_name<F: FnMut(&str)>(f: F) {
                            <dyn #name<#generic_args>>::build_type_name(f);
                        }
                    }

                    impl<__T: #name<#generic_args> + ArchiveDyn, #generic_params> #archive_trait<#generic_args> for __T {}

                    impl<'a, #generic_params> From<TraitObject> for &'a (dyn #name<#generic_args> + 'static) {
                        fn from(trait_object: TraitObject) -> &'a (dyn #name<#generic_args> + 'static) {
                            unsafe { core::mem::transmute(trait_object) }
                        }
                    }

                    impl<'a, #generic_params> From<TraitObject> for &'a mut (dyn #name<#generic_args> + 'static) {
                        fn from(trait_object: TraitObject) -> &'a mut (dyn #name<#generic_args> + 'static) {
                            unsafe { core::mem::transmute(trait_object) }
                        }
                    }

                    impl<#generic_params> Resolve<dyn #archive_trait<#generic_args>> for DynResolver
                    where
                        #type_name_wheres
                    {
                        type Archived = ArchivedDyn<dyn #name<#generic_args>>;

                        fn resolve(self, pos: usize, value: &dyn #archive_trait<#generic_args>) -> Self::Archived {
                            ArchivedDyn::new(pos, self, hash_value(value) | 1)
                        }
                    }

                    impl<#generic_params> ArchiveRef for dyn #archive_trait<#generic_args>
                    where
                        #type_name_wheres
                    {
                        type Archived = dyn #name<#generic_args>;
                        type Reference = ArchivedDyn<dyn #name<#generic_args>>;
                        type Resolver = DynResolver;

                        fn archive_ref<W: Write + ?Sized>(&self, mut writer: &mut W) -> Result<Self::Resolver, W::Error> {
                            self.archive_dyn(&mut writer).map_err(|e| *e.downcast().unwrap())
                        }
                    }
                };
            }
        }
    };

    proc_macro::TokenStream::from(input_impl)
}
