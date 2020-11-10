mod core_impl;
#[cfg(feature = "std")]
mod std_impl;

pub trait TypeName {
    fn build_type_name<F: FnMut(&str)>(f: F);
}
