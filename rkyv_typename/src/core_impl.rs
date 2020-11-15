use crate::TypeName;

macro_rules! impl_primitive {
    ($type:ty) => {
        impl TypeName for $type {
            fn build_type_name<F: FnMut(&'static str)>(mut f: F) {
                f(stringify!($type));
            }
        }
    };
}

impl_primitive!(());
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

macro_rules! impl_tuple {
    ($type:ident,) => {
        impl<$type: TypeName> TypeName for ($type,) {
            fn build_type_name<F: FnMut(&str)>(mut f: F) {
                f("(");
                $type::build_type_name(&mut f);
                f(",)");
            }
        }
    };
    ($first:ident, $($rest:ident,)+) => {
        impl<$first: TypeName, $($rest: TypeName),+> TypeName for ($first, $($rest,)+) {
            fn build_type_name<F: FnMut(&str)>(mut f: F) {
                f("(");
                $first::build_type_name(&mut f);
                $(f(", "); $rest::build_type_name(&mut f);)+
                f(")");
            }
        }

        impl_tuple! { $($rest,)+ }
    };
}

impl_tuple! { T11, T10, T9, T8, T7, T6, T5, T4, T3, T2, T1, T0, }

#[cfg(not(feature = "const_generics"))]
macro_rules! impl_array {
    () => ();
    ($len:literal, $($rest:literal,)*) => {
        impl<T: TypeName> TypeName for [T; $len] {
            fn build_type_name<F: FnMut(&str)>(mut f: F) {
                f("[");
                T::build_type_name(&mut f);
                f("; ");
                f(stringify!($len));
                f("]");
            }
        }

        impl_array! { $($rest,)* }
    };
}

#[cfg(not(feature = "const_generics"))]
impl_array! { 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, }

#[cfg(feature = "const_generics")]
impl<T: TypeName, const N: usize> TypeName for [T; N] {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("[");
        T::build_type_name(&mut f);
        f("; ");
        f(N.to_string().as_str());
        f("]");
    }
}

impl TypeName for str {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("str");
    }
}

impl<T: TypeName> TypeName for [T] {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("[");
        T::build_type_name(&mut f);
        f("]");
    }
}

impl<T: TypeName> TypeName for Option<T> {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("Option<");
        T::build_type_name(&mut f);
        f(">");
    }
}
