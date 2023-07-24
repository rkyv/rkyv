use crate::ConstGeneric;

macro_rules! impl_primitive {
    ($type:ty) => {
        impl ConstGeneric for $type {
            fn build_name<F: FnMut(&str)>(&self, mut f: F) {
                // the debug trait happens to give the right result for this primitive value
                // but this may not be the case for other const generic types that may be introduced
                // into the language
                let const_value = format!("{:?}", self);
                f(&const_value);
            }
        }
    };
}

impl_primitive!(bool);
impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(i128);
impl_primitive!(u8);
impl_primitive!(u16);
impl_primitive!(u32);
impl_primitive!(u64);
impl_primitive!(u128);
impl_primitive!(f32);
impl_primitive!(f64);
impl_primitive!(char);
impl_primitive!(usize);
impl_primitive!(isize);
