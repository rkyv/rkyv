use std::marker::{PhantomData, PhantomPinned};

/// A type that is `Copy` and can be archived without additional processing.
///
/// This trait is similar to `Copy` in that it's automatically implemented for all types composed
/// entirely of other `ArchiveCopy` types. `Copy` is necessary, but not sufficient for `ArchiveCopy`
/// as some `Copy` type representations may vary from platform to platform.
#[rustc_unsafe_specialization_marker]
pub auto trait ArchiveCopy {}

#[cfg(any(
    all(target_endian = "little", feature = "archive_be"),
    all(target_endian = "big", feature = "archive_le"),
))]
const _: () = {
    impl !ArchiveCopy for i16 {}
    impl !ArchiveCopy for i32 {}
    impl !ArchiveCopy for i64 {}
    impl !ArchiveCopy for i128 {}
    impl !ArchiveCopy for u16 {}
    impl !ArchiveCopy for u32 {}
    impl !ArchiveCopy for u64 {}
    impl !ArchiveCopy for u128 {}
    impl !ArchiveCopy for f32 {}
    impl !ArchiveCopy for f64 {}
    impl !ArchiveCopy for char {}
};

impl !ArchiveCopy for isize {}
impl !ArchiveCopy for usize {}

impl<T: ?Sized> !ArchiveCopy for *const T {}
impl<T: ?Sized> !ArchiveCopy for *mut T {}
impl<T: ?Sized> !ArchiveCopy for &T {}
impl<T: ?Sized> !ArchiveCopy for &mut T {}

impl !ArchiveCopy for PhantomPinned {}

/// Types that are `ArchiveCopy` and have no padding.
///
/// These types are always safe to `memcpy` around because they will never contain uninitialized
/// padding.
#[rustc_unsafe_specialization_marker]
pub unsafe trait ArchiveCopySafe: ArchiveCopy + Sized {
    const PACKED_SIZE: usize;
}

unsafe impl ArchiveCopySafe for i8 {}
unsafe impl ArchiveCopySafe for u8 {}
unsafe impl ArchiveCopySafe for bool {}

#[cfg(not(any(
    all(target_endian = "little", feature = "archive_be"),
    all(target_endian = "big", feature = "archive_le"),
)))]
const _: () = {
    unsafe impl ArchiveCopySafe for i16 {}
    unsafe impl ArchiveCopySafe for i32 {}
    unsafe impl ArchiveCopySafe for i64 {}
    unsafe impl ArchiveCopySafe for i128 {}
    unsafe impl ArchiveCopySafe for u16 {}
    unsafe impl ArchiveCopySafe for u32 {}
    unsafe impl ArchiveCopySafe for u64 {}
    unsafe impl ArchiveCopySafe for u128 {}
    unsafe impl ArchiveCopySafe for f32 {}
    unsafe impl ArchiveCopySafe for f64 {}
    unsafe impl ArchiveCopySafe for char {}
};

unsafe impl ArchiveCopySafe for () {}
unsafe impl<T: ?Sized> ArchiveCopySafe for PhantomData<T> {}
unsafe impl<T: ArchiveCopySafe, const N: usize> ArchiveCopySafe for [T; N] {}

macro_rules! impl_tuple {
    () => {};
    (T, $($ts:ident,)*) => {
        unsafe impl<T: ArchiveCopySafe> ArchiveCopySafe for (T, $($ts,)*) {}
        impl_tuple!($($ts,)*);
    };
}

impl_tuple!(T, T, T, T, T, T, T, T, T, T, T,);
