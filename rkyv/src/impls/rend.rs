use rancor::Fallible;

use crate::{
    rend::*, traits::CopyOptimization, Archive, Deserialize, Place, Serialize,
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
            fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<D: Fallible + ?Sized> Deserialize<$type, D> for $type {
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
    use rend::*;

    use crate::api::test::{roundtrip, to_bytes};

    #[test]
    fn roundtrip_integers() {
        roundtrip(&i16_be::from_native(12345i16));
        roundtrip(&i32_be::from_native(1234567890i32));
        roundtrip(&i64_be::from_native(1234567890123456789i64));
        roundtrip(&i128_be::from_native(
            123456789012345678901234567890123456789i128,
        ));
        roundtrip(&u16_be::from_native(12345u16));
        roundtrip(&u32_be::from_native(1234567890u32));
        roundtrip(&u64_be::from_native(12345678901234567890u64));
        roundtrip(&u128_be::from_native(
            123456789012345678901234567890123456789u128,
        ));

        roundtrip(&i16_le::from_native(12345i16));
        roundtrip(&i32_le::from_native(1234567890i32));
        roundtrip(&i64_le::from_native(1234567890123456789i64));
        roundtrip(&i128_le::from_native(
            123456789012345678901234567890123456789i128,
        ));
        roundtrip(&u16_le::from_native(12345u16));
        roundtrip(&u32_le::from_native(1234567890u32));
        roundtrip(&u64_le::from_native(12345678901234567890u64));
        roundtrip(&u128_le::from_native(
            123456789012345678901234567890123456789u128,
        ));
    }

    #[test]
    fn roundtrip_floats() {
        roundtrip(&f32_be::from_native(1234567f32));
        roundtrip(&f64_be::from_native(12345678901234f64));

        roundtrip(&f32_le::from_native(1234567f32));
        roundtrip(&f64_le::from_native(12345678901234f64));
    }

    #[test]
    fn roundtrip_chars() {
        roundtrip(&char_be::from_native('x'));
        roundtrip(&char_be::from_native('🥺'));

        roundtrip(&char_le::from_native('x'));
        roundtrip(&char_le::from_native('🥺'));
    }

    #[test]
    fn roundtrip_nonzero() {
        roundtrip(&NonZeroI16_be::new(12345).unwrap());
        roundtrip(&NonZeroI32_be::new(1234567890).unwrap());
        roundtrip(&NonZeroI64_be::new(1234567890123456789).unwrap());
        roundtrip(
            &NonZeroI128_be::new(123456789012345678901234567890123456789)
                .unwrap(),
        );
        roundtrip(&NonZeroU16_be::new(12345).unwrap());
        roundtrip(&NonZeroU32_be::new(1234567890).unwrap());
        roundtrip(&NonZeroU64_be::new(1234567890123456789).unwrap());
        roundtrip(
            &NonZeroU128_be::new(123456789012345678901234567890123456789)
                .unwrap(),
        );

        roundtrip(&NonZeroI16_le::new(12345).unwrap());
        roundtrip(&NonZeroI32_le::new(1234567890).unwrap());
        roundtrip(&NonZeroI64_le::new(1234567890123456789).unwrap());
        roundtrip(
            &NonZeroI128_le::new(123456789012345678901234567890123456789)
                .unwrap(),
        );
        roundtrip(&NonZeroU16_le::new(12345).unwrap());
        roundtrip(&NonZeroU32_le::new(1234567890).unwrap());
        roundtrip(&NonZeroU64_le::new(1234567890123456789).unwrap());
        roundtrip(
            &NonZeroU128_le::new(123456789012345678901234567890123456789)
                .unwrap(),
        );
    }

    #[test]
    fn verify_endianness() {
        // Big endian
        let value = i32_be::from_native(0x12345678);
        to_bytes(&value, |buf| {
            assert_eq!(&buf[0..4], &[0x12, 0x34, 0x56, 0x78]);
        });

        // Little endian
        let value = i32_le::from_native(0x12345678i32);
        to_bytes(&value, |buf| {
            assert_eq!(&buf[0..4], &[0x78, 0x56, 0x34, 0x12]);
        });
    }
}
