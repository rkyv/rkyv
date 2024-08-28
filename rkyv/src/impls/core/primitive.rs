use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize,
    NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};

use rancor::Fallible;

use crate::{
    primitive::{
        ArchivedChar, ArchivedF32, ArchivedF64, ArchivedI128, ArchivedI16,
        ArchivedI32, ArchivedI64, ArchivedIsize, ArchivedNonZeroI128,
        ArchivedNonZeroI16, ArchivedNonZeroI32, ArchivedNonZeroI64,
        ArchivedNonZeroIsize, ArchivedNonZeroU128, ArchivedNonZeroU16,
        ArchivedNonZeroU32, ArchivedNonZeroU64, ArchivedNonZeroUsize,
        ArchivedU128, ArchivedU16, ArchivedU32, ArchivedU64, ArchivedUsize,
    },
    traits::{CopyOptimization, NoUndef},
    Archive, Deserialize, Place, Portable, Serialize,
};

macro_rules! unsafe_impl_primitive {
    ($($ty:ty),* $(,)?) => {
        $(
            unsafe impl NoUndef for $ty {}
            unsafe impl Portable for $ty {}
        )*
    };
}

unsafe_impl_primitive! {
    (),
    bool,
    i8,
    u8,
    NonZeroI8,
    NonZeroU8,
    rend::NonZeroI16_be,
    rend::NonZeroI16_le,
    rend::NonZeroI32_be,
    rend::NonZeroI32_le,
    rend::NonZeroI64_be,
    rend::NonZeroI64_le,
    rend::NonZeroI128_be,
    rend::NonZeroI128_le,
    rend::NonZeroU16_be,
    rend::NonZeroU16_le,
    rend::NonZeroU32_be,
    rend::NonZeroU32_le,
    rend::NonZeroU64_be,
    rend::NonZeroU64_le,
    rend::NonZeroU128_be,
    rend::NonZeroU128_le,
    rend::char_be,
    rend::char_le,
    rend::f32_be,
    rend::f32_le,
    rend::f64_be,
    rend::f64_le,
    rend::i16_be,
    rend::i16_le,
    rend::i32_be,
    rend::i32_le,
    rend::i64_be,
    rend::i64_le,
    rend::i128_be,
    rend::i128_le,
    rend::u16_be,
    rend::u16_le,
    rend::u32_be,
    rend::u32_le,
    rend::u64_be,
    rend::u64_le,
    rend::u128_be,
    rend::u128_le,
    rend::unaligned::NonZeroI16_ube,
    rend::unaligned::NonZeroI16_ule,
    rend::unaligned::NonZeroI32_ube,
    rend::unaligned::NonZeroI32_ule,
    rend::unaligned::NonZeroI64_ube,
    rend::unaligned::NonZeroI64_ule,
    rend::unaligned::NonZeroI128_ube,
    rend::unaligned::NonZeroI128_ule,
    rend::unaligned::NonZeroU16_ube,
    rend::unaligned::NonZeroU16_ule,
    rend::unaligned::NonZeroU32_ube,
    rend::unaligned::NonZeroU32_ule,
    rend::unaligned::NonZeroU64_ube,
    rend::unaligned::NonZeroU64_ule,
    rend::unaligned::NonZeroU128_ube,
    rend::unaligned::NonZeroU128_ule,
    rend::unaligned::char_ube,
    rend::unaligned::char_ule,
    rend::unaligned::f32_ube,
    rend::unaligned::f32_ule,
    rend::unaligned::f64_ube,
    rend::unaligned::f64_ule,
    rend::unaligned::i16_ube,
    rend::unaligned::i16_ule,
    rend::unaligned::i32_ube,
    rend::unaligned::i32_ule,
    rend::unaligned::i64_ube,
    rend::unaligned::i64_ule,
    rend::unaligned::i128_ube,
    rend::unaligned::i128_ule,
    rend::unaligned::u16_ube,
    rend::unaligned::u16_ule,
    rend::unaligned::u32_ube,
    rend::unaligned::u32_ule,
    rend::unaligned::u64_ube,
    rend::unaligned::u64_ule,
    rend::unaligned::u128_ube,
    rend::unaligned::u128_ule,
}

macro_rules! impl_serialize_noop {
    ($type:ty) => {
        impl<S: Fallible + ?Sized> Serialize<S> for $type {
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }
    };
}

macro_rules! impl_archive_self_primitive {
    ($type:ty) => {
        impl Archive for $type {
            const COPY_OPTIMIZATION: CopyOptimization<Self> =
                unsafe { CopyOptimization::enable() };

            type Archived = Self;
            type Resolver = ();

            #[inline]
            fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
                out.write(*self);
            }
        }

        impl_serialize_noop!($type);

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for $type {
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(*self)
            }
        }
    };
}

macro_rules! impl_archive_self_primitives {
    ($($type:ty;)*) => {
        $(
            impl_archive_self_primitive!($type);
        )*
    }
}

impl_archive_self_primitives! {
    ();
    bool;
    i8;
    u8;
    NonZeroI8;
    NonZeroU8;
}

#[cfg(any(
    all(not(feature = "big_endian"), target_endian = "little"),
    all(feature = "big_endian", target_endian = "big"),
))]
const MULTIBYTE_PRIMITIVES_ARE_TRIVIALLY_COPYABLE: bool = true;
#[cfg(any(
    all(feature = "big_endian", target_endian = "little"),
    all(not(feature = "big_endian"), target_endian = "big"),
))]
const MULTIBYTE_PRIMITIVES_ARE_TRIVIALLY_COPYABLE: bool = false;

macro_rules! impl_multibyte_primitive {
    ($archived:ident : $type:ty) => {
        impl Archive for $type {
            const COPY_OPTIMIZATION: CopyOptimization<Self> = unsafe {
                CopyOptimization::enable_if(
                    MULTIBYTE_PRIMITIVES_ARE_TRIVIALLY_COPYABLE,
                )
            };

            type Archived = $archived;
            type Resolver = ();

            #[inline]
            fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
                out.write(<$archived>::from_native(*self));
            }
        }

        impl_serialize_noop!($type);

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for $archived {
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(self.to_native())
            }
        }
    };
}

macro_rules! impl_multibyte_primitives {
    ($($archived:ident: $type:ty),* $(,)?) => {
        $(
            impl_multibyte_primitive!($archived: $type);
        )*
    };
}

impl_multibyte_primitives! {
    ArchivedI16: i16,
    ArchivedI32: i32,
    ArchivedI64: i64,
    ArchivedI128: i128,
    ArchivedU16: u16,
    ArchivedU32: u32,
    ArchivedU64: u64,
    ArchivedU128: u128,
    ArchivedF32: f32,
    ArchivedF64: f64,
    ArchivedChar: char,
    ArchivedNonZeroI16: NonZeroI16,
    ArchivedNonZeroI32: NonZeroI32,
    ArchivedNonZeroI64: NonZeroI64,
    ArchivedNonZeroI128: NonZeroI128,
    ArchivedNonZeroU16: NonZeroU16,
    ArchivedNonZeroU32: NonZeroU32,
    ArchivedNonZeroU64: NonZeroU64,
    ArchivedNonZeroU128: NonZeroU128,
}

// usize

#[cfg(any(
    all(target_pointer_width = "16", feature = "pointer_width_16"),
    all(
        target_pointer_width = "32",
        not(any(feature = "pointer_width_16", feature = "pointer_width_64")),
    ),
    all(target_pointer_width = "64", feature = "pointer_width_64"),
))]
const POINTER_WIDTH_EQUALS_ARCHIVED_POINTER_WIDTH: bool = true;
#[cfg(not(any(
    all(target_pointer_width = "16", feature = "pointer_width_16"),
    all(
        target_pointer_width = "32",
        not(any(feature = "pointer_width_16", feature = "pointer_width_64")),
    ),
    all(target_pointer_width = "64", feature = "pointer_width_64"),
)))]
const POINTER_WIDTH_EQUALS_ARCHIVED_POINTER_WIDTH: bool = false;

impl Archive for usize {
    const COPY_OPTIMIZATION: CopyOptimization<Self> = unsafe {
        CopyOptimization::enable_if(
            MULTIBYTE_PRIMITIVES_ARE_TRIVIALLY_COPYABLE
                && POINTER_WIDTH_EQUALS_ARCHIVED_POINTER_WIDTH,
        )
    };

    type Archived = ArchivedUsize;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        out.write(ArchivedUsize::from_native(*self as _));
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for usize {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<usize, D> for ArchivedUsize {
    fn deserialize(&self, _: &mut D) -> Result<usize, D::Error> {
        Ok(self.to_native() as usize)
    }
}

// isize

impl Archive for isize {
    const COPY_OPTIMIZATION: CopyOptimization<Self> = unsafe {
        CopyOptimization::enable_if(
            MULTIBYTE_PRIMITIVES_ARE_TRIVIALLY_COPYABLE
                && POINTER_WIDTH_EQUALS_ARCHIVED_POINTER_WIDTH,
        )
    };

    type Archived = ArchivedIsize;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        out.write(ArchivedIsize::from_native(*self as _));
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for isize {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<isize, D> for ArchivedIsize {
    fn deserialize(&self, _: &mut D) -> Result<isize, D::Error> {
        Ok(self.to_native() as isize)
    }
}

// NonZeroUsize

impl Archive for NonZeroUsize {
    const COPY_OPTIMIZATION: CopyOptimization<Self> = unsafe {
        CopyOptimization::enable_if(
            MULTIBYTE_PRIMITIVES_ARE_TRIVIALLY_COPYABLE
                && POINTER_WIDTH_EQUALS_ARCHIVED_POINTER_WIDTH,
        )
    };

    type Archived = ArchivedNonZeroUsize;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        let value =
            unsafe { ArchivedNonZeroUsize::new_unchecked(self.get() as _) };
        out.write(value);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for NonZeroUsize {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D> Deserialize<NonZeroUsize, D> for ArchivedNonZeroUsize
where
    D: Fallible + ?Sized,
{
    fn deserialize(&self, _: &mut D) -> Result<NonZeroUsize, D::Error> {
        Ok(unsafe { NonZeroUsize::new_unchecked(self.get() as usize) })
    }
}

// NonZeroIsize

impl Archive for NonZeroIsize {
    const COPY_OPTIMIZATION: CopyOptimization<Self> = unsafe {
        CopyOptimization::enable_if(
            MULTIBYTE_PRIMITIVES_ARE_TRIVIALLY_COPYABLE
                && POINTER_WIDTH_EQUALS_ARCHIVED_POINTER_WIDTH,
        )
    };

    type Archived = ArchivedNonZeroIsize;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        let value =
            unsafe { ArchivedNonZeroIsize::new_unchecked(self.get() as _) };
        out.write(value);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for NonZeroIsize {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D> Deserialize<NonZeroIsize, D> for ArchivedNonZeroIsize
where
    D: Fallible + ?Sized,
{
    fn deserialize(&self, _: &mut D) -> Result<NonZeroIsize, D::Error> {
        Ok(unsafe { NonZeroIsize::new_unchecked(self.get() as isize) })
    }
}

#[cfg(test)]
mod tests {
    use core::num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    };

    use crate::api::test::{roundtrip, roundtrip_with};

    #[test]
    fn roundtrip_portable_primitives() {
        roundtrip(&());
        roundtrip(&true);
        roundtrip(&false);
        roundtrip(&123i8);
        roundtrip(&123u8);
        roundtrip(&NonZeroI8::new(123i8).unwrap());
        roundtrip(&NonZeroU8::new(123u8).unwrap());
    }

    #[test]
    fn roundtrip_multibyte_primitives() {
        roundtrip(&12345i16);
        roundtrip(&1234567890i32);
        roundtrip(&1234567890123456789i64);
        roundtrip(&123456789012345678901234567890123456789i128);
        roundtrip(&12345u16);
        roundtrip(&1234567890u32);
        roundtrip(&12345678901234567890u64);
        roundtrip(&123456789012345678901234567890123456789u128);

        roundtrip(&1234567f32);
        roundtrip(&12345678901234f64);

        roundtrip(&'x');
        roundtrip(&'🥺');

        roundtrip(&NonZeroI16::new(12345i16).unwrap());
        roundtrip(&NonZeroI32::new(1234567890i32).unwrap());
        roundtrip(&NonZeroI64::new(1234567890123456789i64).unwrap());
        roundtrip(
            &NonZeroI128::new(123456789012345678901234567890123456789i128)
                .unwrap(),
        );
        roundtrip(&NonZeroU16::new(12345u16).unwrap());
        roundtrip(&NonZeroU32::new(1234567890u32).unwrap());
        roundtrip(&NonZeroU64::new(12345678901234567890u64).unwrap());
        roundtrip(
            &NonZeroU128::new(123456789012345678901234567890123456789u128)
                .unwrap(),
        );
    }

    #[test]
    fn roundtrip_sizes() {
        roundtrip_with(&12345isize, |a, b| {
            assert_eq!(*a, isize::try_from(b.to_native()).unwrap())
        });
        roundtrip_with(&12345usize, |a, b| {
            assert_eq!(*a, usize::try_from(b.to_native()).unwrap())
        });
        roundtrip_with(&NonZeroIsize::new(12345isize).unwrap(), |a, b| {
            assert_eq!(*a, NonZeroIsize::try_from(b.to_native()).unwrap())
        });
        roundtrip_with(&NonZeroUsize::new(12345usize).unwrap(), |a, b| {
            assert_eq!(*a, NonZeroUsize::try_from(b.to_native()).unwrap())
        });
    }
}
