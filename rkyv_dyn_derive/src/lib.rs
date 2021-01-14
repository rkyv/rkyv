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

struct Args {
    archive: Option<LitStr>,
    unarchive: Option<Option<LitStr>>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        mod kw {
            syn::custom_keyword!(name);
            syn::custom_keyword!(unarchive);
        }

        let mut archive = None;
        let mut unarchive = None;

        let mut needs_punct = false;
        while !input.is_empty() {
            if needs_punct {
                input.parse::<Token![,]>()?;
            }

            if input.peek(kw::name) {
                if archive.is_some() {
                    return Err(input.error("duplicate name argument"));
                }

                input.parse::<kw::name>()?;
                input.parse::<Token![=]>()?;
                archive = Some(input.parse::<LitStr>()?);
            } else if input.peek(kw::unarchive) {
                if unarchive.is_some() {
                    return Err(input.error("duplicate unarchive argument"));
                }

                input.parse::<kw::unarchive>()?;
                if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    unarchive = Some(Some(input.parse::<LitStr>()?));
                } else {
                    unarchive = Some(None);
                }
            } else {
                return Err(
                    input.error("expected name = \"...\" or unarchive = \"...\" parameters")
                );
            }

            needs_punct = true;
        }

        Ok(Args { archive, unarchive })
    }
}

/// Creates archiveable trait objects and registers implementations.
///
/// Prepend to trait definitions and implementations. For generic
/// implementations, you may need to manually register impls with the trait
/// object system. See `register_impl` for more information.
///
/// See `ArchiveDyn` for usage information and examples.
///
/// # Parameters
///
/// - `name = "..."`: Chooses the name of the archive trait. By default, it will
/// be named "Archive" + your trait name.
/// - `unarchive`, `unarchive = "..."`: Adds unarchive support to the archived
/// trait. Similarly to the `name` parameter, you can choose the name of the
/// unarchive trait and by default it will be named "Unarchive" + your trait
/// name.
#[proc_macro_attribute]
pub fn archive_dyn(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as Input);

    let args = parse_macro_input!(attr as Args);

    let input_impl = match input {
        Input::Impl(ref input) => {
            if !input.generics.params.is_empty() {
                Error::new(input.generics.span(), "#[archive_dyn] can only register non-generic impls; call register_impl! with the concrete types to register and manually implement UnarchiveDyn for archived types if necessary").to_compile_error()
            } else if let Some((_, ref trait_, _)) = input.trait_ {
                let ty = &input.self_ty;

                let archive_trait = if let Some(ar_name) = args.archive {
                    let mut path = trait_.clone();
                    let last = path.segments.last_mut().unwrap();
                    last.ident = Ident::new(&ar_name.value(), ar_name.span());
                    path
                } else {
                    let mut path = trait_.clone();
                    let last = path.segments.last_mut().unwrap();
                    last.ident = Ident::new(&format!("Archive{}", last.ident), trait_.span());
                    path
                };

                let (unarchive_trait, unarchive_impl) = if let Some(unarchive) = args.unarchive {
                    let unarchive_trait = if let Some(ua_name) = unarchive {
                        let mut path = trait_.clone();
                        let last = path.segments.last_mut().unwrap();
                        last.ident = Ident::new(&ua_name.value(), ua_name.span());
                        path
                    } else {
                        let mut path = trait_.clone();
                        let last = path.segments.last_mut().unwrap();
                        last.ident = Ident::new(&format!("Unarchive{}", last.ident), trait_.span());
                        path
                    };

                    (
                        unarchive_trait,
                        quote! {
                            impl UnarchiveDyn<dyn #archive_trait> for Archived<#ty>
                            where
                                Archived<#ty>: Unarchive<#ty>,
                            {
                                unsafe fn unarchive_dyn(&self, alloc: unsafe fn(core::alloc::Layout) -> *mut u8) -> *mut dyn #archive_trait {
                                    let result = alloc(core::alloc::Layout::new::<#ty>()) as *mut #ty;
                                    result.write(self.unarchive());
                                    result as *mut dyn #archive_trait
                                }
                            }
                        },
                    )
                } else {
                    (trait_.clone(), quote! {})
                };

                quote! {
                    #input

                    const _: () = {
                        use rkyv::{Archived, Unarchive};
                        use rkyv_dyn::UnarchiveDyn;

                        rkyv_dyn::register_impl!(Archived<#ty> as dyn #unarchive_trait);

                        #unarchive_impl
                    };
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
            let archive_trait = args
                .archive
                .map(|ar_name| Ident::new(&ar_name.value(), ar_name.span()))
                .unwrap_or_else(|| Ident::new(&format!("Archive{}", name), name.span()));

            let type_name_wheres = input.generics.type_params().map(|p| {
                let name = &p.ident;
                quote_spanned! { name.span() => #name: TypeName }
            });
            let type_name_wheres = quote! { #(#type_name_wheres,)* };

            let (unarchive_trait, unarchive_trait_def, unarchive_trait_impl) = if let Some(
                unarchive,
            ) = args.unarchive
            {
                let unarchive_trait = if let Some(ua_name) = unarchive {
                    Ident::new(&ua_name.value(), ua_name.span())
                } else {
                    Ident::new(&format!("Unarchive{}", name), name.span())
                };

                (
                    unarchive_trait.clone(),
                    quote! {
                        #vis trait #unarchive_trait<#generic_params>: #name<#generic_args> + rkyv_dyn::UnarchiveDyn<dyn #archive_trait<#generic_args>> {}
                    },
                    quote! {

                        impl<__T: #name<#generic_args> + UnarchiveDyn<dyn #archive_trait<#generic_args>>, #generic_params> #unarchive_trait<#generic_args> for __T {}

                        impl<#generic_params> UnarchiveRef<dyn #archive_trait<#generic_args>> for ArchivedDyn<dyn #unarchive_trait<#generic_args>> {
                            unsafe fn unarchive_ref(&self, alloc: unsafe fn(core::alloc::Layout) -> *mut u8) -> *mut dyn #archive_trait<#generic_args> {
                                (*self).unarchive_dyn(alloc)
                            }
                        }
                    },
                )
            } else {
                (name.clone(), quote! {}, quote! {})
            };

            let build_type_name = if !input.generics.params.is_empty() {
                let dyn_name = format!("dyn {}<", unarchive_trait);
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
                quote! { f(stringify!(dyn #unarchive_trait)); }
            };

            quote! {
                #input

                #vis trait #archive_trait<#generic_params>: #name<#generic_args> + rkyv_dyn::ArchiveDyn {}

                #unarchive_trait_def

                const _: ()  = {
                    use rkyv::{
                        Archived,
                        ArchiveRef,
                        Resolve,
                        UnarchiveRef,
                        Write,
                    };
                    use rkyv_dyn::{
                        ArchiveDyn,
                        ArchivedDyn,
                        DynError,
                        DynResolver,
                        RegisteredImpl,
                        UnarchiveDyn,
                        WriteDyn,
                    };
                    use rkyv_typename::TypeName;

                    impl<__T: #name<#generic_args> + ArchiveDyn + Archive, #generic_params> #archive_trait<#generic_args> for __T
                    where
                        __T::Archived: RegisteredImpl<dyn #unarchive_trait<#generic_args>>
                    {}

                    #unarchive_trait_impl

                    impl<#generic_params> TypeName for dyn #unarchive_trait<#generic_args> + '_
                    where
                        #type_name_wheres
                    {
                        fn build_type_name<F: FnMut(&str)>(mut f: F) {
                            #build_type_name
                        }
                    }

                    impl<#generic_params> Resolve<dyn #archive_trait<#generic_args>> for DynResolver {
                        type Archived = ArchivedDyn<dyn #unarchive_trait<#generic_args>>;

                        fn resolve(self, pos: usize, _: &dyn #archive_trait<#generic_args>) -> Self::Archived {
                            ArchivedDyn::resolve(pos, self)
                        }
                    }

                    impl<#generic_params> ArchiveRef for dyn #archive_trait<#generic_args> {
                        type Archived = dyn #unarchive_trait<#generic_args>;
                        type Reference = ArchivedDyn<dyn #unarchive_trait<#generic_args>>;
                        type Resolver = DynResolver;

                        fn archive_ref<W: Write + ?Sized>(&self, mut writer: &mut W) -> Result<Self::Resolver, W::Error> {
                            self.archive_dyn(&mut writer).map_err(|e| *e.downcast::<W::Error>().unwrap())
                        }
                    }
                };
            }
        }
    };

    proc_macro::TokenStream::from(input_impl)
}
