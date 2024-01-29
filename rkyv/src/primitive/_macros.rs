#[rustfmt::skip]
macro_rules! define_archived_type_alias {
    ($archived:ident: $name:ident, $ty:ty) => {
        #[doc = concat!(
            "The archived version of `",
            stringify!($name),
            "`.",
        )]
        pub type $archived = $ty;
    };
}

macro_rules! define_archived_primitive {
    ($archived:ident: $name:ident, $le:ty, $be:ty) => {
        #[cfg(not(feature = "big_endian"))]
        define_archived_type_alias!($archived: $name, $le);
        #[cfg(feature = "big_endian")]
        define_archived_type_alias!($archived: $name, $be);
    }
}
