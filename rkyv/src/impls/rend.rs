use crate::{rend::*, Archive, Archived, Deserialize, Fallible, Serialize};

macro_rules! impl_rend_primitive {
    ($type:ty) => {
        impl Archive for $type {
            type Archived = Self;
            type Resolver = ();

            #[inline]
            unsafe fn resolve(
                &self,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                out.write(*self);
            }
        }

        // Safety: rend primitives always have the same representation archived and unarchived and
        // contain no padding
        #[cfg(feature = "copy")]
        unsafe impl crate::copy::ArchiveCopySafe for $type {}

        impl<S: Fallible + ?Sized> Serialize<S> for $type {
            #[inline]
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for Archived<$type> {
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
    use crate::{
        archived_root, ser::serializers::CoreSerializer, ser::Serializer,
        Deserialize, Infallible, Serialize,
    };
    use core::fmt;

    type DefaultSerializer = CoreSerializer<256, 256>;

    fn test_archive<T>(value: &T)
    where
        T: fmt::Debug + PartialEq + Serialize<DefaultSerializer>,
        T::Archived: fmt::Debug + PartialEq<T> + Deserialize<T, Infallible>,
    {
        let mut serializer = DefaultSerializer::default();
        serializer
            .serialize_value(value)
            .expect("failed to archive value");
        let len = serializer.pos();
        let buffer = serializer.into_serializer().into_inner();

        let archived_value = unsafe { archived_root::<T>(&buffer[0..len]) };
        assert_eq!(archived_value, value);
        let mut deserializer = Infallible;
        assert_eq!(
            &archived_value.deserialize(&mut deserializer).unwrap(),
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
        use crate::{
            rend::{i32_be, i32_le},
            ser::Serializer,
        };

        // Big endian
        let value = i32_be::from_native(0x12345678);

        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let buf = serializer.into_serializer().into_inner();

        assert_eq!(&buf[0..4], &[0x12, 0x34, 0x56, 0x78]);

        // Little endian
        let value = i32_le::from_native(0x12345678i32);

        let mut serializer = DefaultSerializer::default();
        serializer.serialize_value(&value).unwrap();
        let buf = serializer.into_serializer().into_inner();

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
