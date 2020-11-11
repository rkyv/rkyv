#![cfg_attr(feature = "const_generics", allow(incomplete_features))]
#![cfg_attr(feature = "const_generics", feature(const_generics))]

mod core_impl;
#[cfg(feature = "std")]
mod std_impl;

pub use type_name_derive::TypeName;

pub trait TypeName {
    fn build_type_name<F: FnMut(&str)>(f: F);
}

#[cfg(test)]
mod tests {
    use crate as type_name;
    use crate::TypeName;

    fn type_name_string<T: TypeName>() -> String {
        let mut result = String::new();
        T::build_type_name(|piece| result += piece);
        result
    }

    #[test]
    fn builtin_types() {
        assert_eq!(type_name_string::<[[u8; 4]; 8]>(), "[[u8; 4]; 8]");
        assert_eq!(type_name_string::<Option<[String; 1]>>(), "Option<[String; 1]>");
        assert_eq!(type_name_string::<Option<[Option<u8>; 4]>>(), "Option<[Option<u8>; 4]>");
    }

    #[test]
    fn derive() {
        #[derive(TypeName)]
        struct Test;

        assert_eq!(type_name_string::<Test>(), "Test");
    }

    #[test]
    fn derive_generic() {
        #[derive(TypeName)]
        struct Test<T, U, V>(T, U, V);

        assert_eq!(type_name_string::<Test<u8, [i32; 4], Option<String>>>(), "Test<u8, [i32; 4], Option<String>>");
    }
}
