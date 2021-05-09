use proc_macro2::{Literal, Punct, Spacing, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{spanned::Spanned, Error, Path};

#[derive(Clone, Copy)]
pub enum IntRepr {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl ToTokens for IntRepr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::I8 => tokens.append_all(quote! { #[repr(i8)] }),
            Self::I16 => tokens.append_all(quote! { #[repr(i16)] }),
            Self::I32 => tokens.append_all(quote! { #[repr(i32)] }),
            Self::I64 => tokens.append_all(quote! { #[repr(i64)] }),
            Self::I128 => tokens.append_all(quote! { #[repr(i128)] }),
            Self::U8 => tokens.append_all(quote! { #[repr(u8)] }),
            Self::U16 => tokens.append_all(quote! { #[repr(u16)] }),
            Self::U32 => tokens.append_all(quote! { #[repr(u32)] }),
            Self::U64 => tokens.append_all(quote! { #[repr(u64)] }),
            Self::U128 => tokens.append_all(quote! { #[repr(u128)] }),
        }
    }
}

impl IntRepr {
    #[inline]
    #[cfg(not(feature = "arbitrary_enum_discriminant"))]
    pub fn enum_discriminant(&self, _: usize) -> Option<EnumDiscriminant> {
        None
    }

    #[inline]
    #[cfg(feature = "arbitrary_enum_discriminant")]
    pub fn enum_discriminant(&self, index: usize) -> Option<EnumDiscriminant> {
        #[cfg(not(any(
            all(target_endian = "little", feature = "archive_be"),
            all(target_endian = "big", feature = "archive_le"),
        )))]
        let value = index as u128;

        #[cfg(any(
            all(target_endian = "little", feature = "archive_be"),
            all(target_endian = "big", feature = "archive_le"),
        ))]
        let value = match self {
            Self::I8 => (index as i8).swap_bytes() as u128,
            Self::I16 => (index as i16).swap_bytes() as u128,
            Self::I32 => (index as i32).swap_bytes() as u128,
            Self::I64 => (index as i64).swap_bytes() as u128,
            Self::I128 => (index as i128).swap_bytes() as u128,
            Self::U8 => (index as u8).swap_bytes() as u128,
            Self::U16 => (index as u16).swap_bytes() as u128,
            Self::U32 => (index as u32).swap_bytes() as u128,
            Self::U64 => (index as u64).swap_bytes() as u128,
            Self::U128 => (index as u128).swap_bytes(),
        };

        Some(EnumDiscriminant { repr: *self, value })
    }
}

// None of these variants are constructed unless the arbitrary_enum_discriminant feature is enabled
#[allow(dead_code)]
pub struct EnumDiscriminant {
    repr: IntRepr,
    value: u128,
}

impl ToTokens for EnumDiscriminant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Punct::new('=', Spacing::Alone));
        tokens.append(match self.repr {
            IntRepr::I8 => Literal::i8_suffixed(self.value as i8),
            IntRepr::I16 => Literal::i16_suffixed(self.value as i16),
            IntRepr::I32 => Literal::i32_suffixed(self.value as i32),
            IntRepr::I64 => Literal::i64_suffixed(self.value as i64),
            IntRepr::I128 => Literal::i128_suffixed(self.value as i128),
            IntRepr::U8 => Literal::u8_suffixed(self.value as u8),
            IntRepr::U16 => Literal::u16_suffixed(self.value as u16),
            IntRepr::U32 => Literal::u32_suffixed(self.value as u32),
            IntRepr::U64 => Literal::u64_suffixed(self.value as u64),
            IntRepr::U128 => Literal::u128_suffixed(self.value),
        });
    }
}

#[derive(Clone, Copy)]
pub enum Repr {
    Rust,
    Transparent,
    Packed,
    C,
    Int(IntRepr),
}

impl ToTokens for Repr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Rust => tokens.append_all(quote! { #[repr(rust)] }),
            Self::Transparent => tokens.append_all(quote! { #[repr(transparent)] }),
            Self::Packed => tokens.append_all(quote! { #[repr(packed)] }),
            Self::C => tokens.append_all(quote! { #[repr(C)] }),
            Self::Int(repr) => tokens.append_all(quote! { #repr }),
        }
    }
}

pub struct ReprAttr {
    pub repr: Repr,
    pub span: Span,
}

impl ReprAttr {
    pub fn try_from_path(path: &Path) -> Result<Self, Error> {
        let repr = if path.is_ident("Rust") {
            Repr::Rust
        } else if path.is_ident("transparent") {
            Repr::Transparent
        } else if path.is_ident("packed") {
            Repr::Packed
        } else if path.is_ident("C") {
            Repr::C
        } else if path.is_ident("i8") {
            Repr::Int(IntRepr::I8)
        } else if path.is_ident("i16") {
            Repr::Int(IntRepr::I16)
        } else if path.is_ident("i32") {
            Repr::Int(IntRepr::I32)
        } else if path.is_ident("i64") {
            Repr::Int(IntRepr::I64)
        } else if path.is_ident("i128") {
            Repr::Int(IntRepr::I128)
        } else if path.is_ident("u8") {
            Repr::Int(IntRepr::U8)
        } else if path.is_ident("u16") {
            Repr::Int(IntRepr::U16)
        } else if path.is_ident("u32") {
            Repr::Int(IntRepr::U32)
        } else if path.is_ident("u64") {
            Repr::Int(IntRepr::U64)
        } else if path.is_ident("u128") {
            Repr::Int(IntRepr::U128)
        } else {
            return Err(Error::new_spanned(path, "invalid repr"));
        };

        Ok(ReprAttr {
            repr,
            span: path.span(),
        })
    }
}

impl ToTokens for ReprAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let repr = &self.repr;
        tokens.append_all(quote_spanned! { self.span => #repr });
    }
}
