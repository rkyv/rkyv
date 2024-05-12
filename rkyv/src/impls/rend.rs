use rancor::Fallible;

use crate::{
    rend::*, Archive, CopyOptimization, Deserialize, Place, Serialize,
};

macro_rules! impl_rend_primitive {
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

        impl<S: Fallible + ?Sized> Serialize<S> for $type {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for $type {
            #[inline]
            fn deserialize(&self, _: &mut D) -> Result<$type, D::Error> {
                Ok(*self)
            }
        }
    };
}

macro_rules! impl_rend_primitives {
    ($($type:ty),* $(,)?) => {
        $(impl_rend_primitive!($type);)*
    };
}

impl_rend_primitives!(
    i16_le,
    i32_le,
    i64_le,
    i128_le,
    u16_le,
    u32_le,
    u64_le,
    u128_le,
    f32_le,
    f64_le,
    char_le,
    NonZeroI16_le,
    NonZeroI32_le,
    NonZeroI64_le,
    NonZeroI128_le,
    NonZeroU16_le,
    NonZeroU32_le,
    NonZeroU64_le,
    NonZeroU128_le,
    i16_be,
    i32_be,
    i64_be,
    i128_be,
    u16_be,
    u32_be,
    u64_be,
    u128_be,
    f32_be,
    f64_be,
    char_be,
    NonZeroI16_be,
    NonZeroI32_be,
    NonZeroI64_be,
    NonZeroI128_be,
    NonZeroU16_be,
    NonZeroU32_be,
    NonZeroU64_be,
    NonZeroU128_be,
);

#[cfg(test)]
mod tests {
    use core::fmt;

    use rancor::{Error, Strategy};

    use crate::{
        access_unchecked, deserialize, ser::DefaultSerializer, to_bytes,
        Deserialize, Serialize,
    };

    fn test_archive<T>(value: &T)
    where
        T: fmt::Debug
            + PartialEq
            + for<'a> Serialize<DefaultSerializer<'a, Error>>,
        T::Archived:
            fmt::Debug + PartialEq<T> + Deserialize<T, Strategy<(), Error>>,
    {
        let bytes = to_bytes::<Error>(value).unwrap();

        let archived_value = unsafe { access_unchecked::<T::Archived>(&bytes) };
        assert_eq!(archived_value, value);
        assert_eq!(
            &deserialize::<T, _, Error>(archived_value, &mut ()).unwrap(),
            value
        );
    }

    #[test]
    fn archive_rend() {
        use crate::rend::*;

        test_archive(&f32_be::from_native(1234567f32));
        test_archive(&f64_be::from_native(12345678901234f64));
        test_archive(&i16_be::from_native(12345i16));
        test_archive(&i32_be::from_native(1234567890i32));
        test_archive(&i64_be::from_native(1234567890123456789i64));
        test_archive(&i128_be::from_native(
            123456789012345678901234567890123456789i128,
        ));
        test_archive(&u16_be::from_native(12345u16));
        test_archive(&u32_be::from_native(1234567890u32));
        test_archive(&u64_be::from_native(12345678901234567890u64));
        test_archive(&u128_be::from_native(
            123456789012345678901234567890123456789u128,
        ));

        test_archive(&f32_le::from_native(1234567f32));
        test_archive(&f64_le::from_native(12345678901234f64));
        test_archive(&i16_le::from_native(12345i16));
        test_archive(&i32_le::from_native(1234567890i32));
        test_archive(&i64_le::from_native(1234567890123456789i64));
        test_archive(&i128_le::from_native(
            123456789012345678901234567890123456789i128,
        ));
        test_archive(&u16_le::from_native(12345u16));
        test_archive(&u32_le::from_native(1234567890u32));
        test_archive(&u64_le::from_native(12345678901234567890u64));
        test_archive(&u128_le::from_native(
            123456789012345678901234567890123456789u128,
        ));
    }

    #[test]
    fn archive_rend_endianness() {
        // Check representations to make sure endianness is preserved
        use crate::rend::{i32_be, i32_le};

        // Big endian
        let value = i32_be::from_native(0x12345678);
        let buf = to_bytes::<Error>(&value).unwrap();
        assert_eq!(&buf[0..4], &[0x12, 0x34, 0x56, 0x78]);

        // Little endian
        let value = i32_le::from_native(0x12345678i32);
        let buf = to_bytes::<Error>(&value).unwrap();
        assert_eq!(&buf[0..4], &[0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn archive_rend_nonzero() {
        use crate::rend::*;

        unsafe {
            test_archive(&NonZeroI16_be::new_unchecked(12345));
            test_archive(&NonZeroI32_be::new_unchecked(1234567890));
            test_archive(&NonZeroI64_be::new_unchecked(1234567890123456789));
            test_archive(&NonZeroI128_be::new_unchecked(
                123456789012345678901234567890123456789,
            ));
            test_archive(&NonZeroU16_be::new_unchecked(12345));
            test_archive(&NonZeroU32_be::new_unchecked(1234567890));
            test_archive(&NonZeroU64_be::new_unchecked(1234567890123456789));
            test_archive(&NonZeroU128_be::new_unchecked(
                123456789012345678901234567890123456789,
            ));

            test_archive(&NonZeroI16_le::new_unchecked(12345));
            test_archive(&NonZeroI32_le::new_unchecked(1234567890));
            test_archive(&NonZeroI64_le::new_unchecked(1234567890123456789));
            test_archive(&NonZeroI128_le::new_unchecked(
                123456789012345678901234567890123456789,
            ));
            test_archive(&NonZeroU16_le::new_unchecked(12345));
            test_archive(&NonZeroU32_le::new_unchecked(1234567890));
            test_archive(&NonZeroU64_le::new_unchecked(1234567890123456789));
            test_archive(&NonZeroU128_le::new_unchecked(
                123456789012345678901234567890123456789,
            ));
        }
    }
}
