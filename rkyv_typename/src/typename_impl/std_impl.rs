use crate::TypeName;

impl TypeName for String {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("alloc::string::String");
    }
}

impl<T: TypeName> TypeName for Box<T> {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("alloc::boxed::Box<");
        T::build_type_name(&mut f);
        f(">");
    }
}

impl<T: TypeName> TypeName for Vec<T> {
    fn build_type_name<F: FnMut(&str)>(mut f: F) {
        f("alloc::vec::Vec<");
        T::build_type_name(&mut f);
        f(">");
    }
}
