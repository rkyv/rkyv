use crate::TypeName;

impl TypeName for String {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("String");
    }
}

impl<T: TypeName> TypeName for Box<T> {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("Box<");
        T::build_type_name(&mut f);
        f(">");
    }
}

impl<T: TypeName> TypeName for Vec<T> {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("Vec<");
        T::build_type_name(&mut f);
        f(">");
    }
}
