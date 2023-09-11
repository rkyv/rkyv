//! Optimization primitives for copyable types.

#[cfg(target_has_atomic = "8")]
use core::sync::atomic::{AtomicBool, AtomicI8, AtomicU8};
#[cfg(target_has_atomic = "16")]
use core::sync::atomic::{AtomicI16, AtomicU16};
#[cfg(target_has_atomic = "32")]
use core::sync::atomic::{AtomicI32, AtomicU32};
#[cfg(target_has_atomic = "64")]
use core::sync::atomic::{AtomicI64, AtomicU64};
use core::{
    marker::{PhantomData, PhantomPinned},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
};

/// A type that is `Copy` and can be archived without additional processing.
///
/// This trait is similar to `Copy` in that it's automatically implemented for all types composed
/// entirely of other `ArchiveCopy` types. `Copy` is necessary, but not sufficient for `ArchiveCopy`
/// as some `Copy` type representations may vary from platform to platform.
#[rustc_unsafe_specialization_marker]
pub auto trait ArchiveCopy {}

// (), PhantomData, PhantomPinned, bool, i8, u8, NonZeroI8, and NonZeroU8 are always ArchiveCopy
impl<T: ?Sized> ArchiveCopy for PhantomData<T> {}

// Multibyte integers are not ArchiveCopy if the target does not match the archive endianness
#[cfg(any(
    all(target_endian = "little", feature = "big_endian"),
    all(target_endian = "big", feature = "little_endian"),
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
    impl !ArchiveCopy for NonZeroI16 {}
    impl !ArchiveCopy for NonZeroI32 {}
    impl !ArchiveCopy for NonZeroI64 {}
    impl !ArchiveCopy for NonZeroI128 {}
    impl !ArchiveCopy for NonZeroU16 {}
    impl !ArchiveCopy for NonZeroU32 {}
    impl !ArchiveCopy for NonZeroU64 {}
    impl !ArchiveCopy for NonZeroU128 {}
};

// Atomics are not `ArchiveCopy`
#[cfg(target_has_atomic = "8")]
const _: () = {
    impl !ArchiveCopy for AtomicBool {}
    impl !ArchiveCopy for AtomicI8 {}
    impl !ArchiveCopy for AtomicU8 {}
};
#[cfg(target_has_atomic = "16")]
const _: () = {
    impl !ArchiveCopy for AtomicI16 {}
    impl !ArchiveCopy for AtomicU16 {}
};
#[cfg(target_has_atomic = "32")]
const _: () = {
    impl !ArchiveCopy for AtomicI32 {}
    impl !ArchiveCopy for AtomicU32 {}
};
#[cfg(target_has_atomic = "64")]
const _: () = {
    impl !ArchiveCopy for AtomicI64 {}
    impl !ArchiveCopy for AtomicU64 {}
};

// Pointers and references are never ArchiveCopy
impl<T: ?Sized> !ArchiveCopy for *const T {}
impl<T: ?Sized> !ArchiveCopy for *mut T {}
impl<T: ?Sized> !ArchiveCopy for &T {}
impl<T: ?Sized> !ArchiveCopy for &mut T {}

impl !ArchiveCopy for crate::RawRelPtr {}

/// Types that are `ArchiveCopy` and have no padding.
///
/// These types are always safe to `memcpy` around because they will never contain uninitialized
/// padding.
#[rustc_unsafe_specialization_marker]
pub unsafe trait ArchiveCopySafe: ArchiveCopy + Sized {}

// (), PhantomData, PhantomPinned, bool, i8, u8, NonZeroI8, and NonZeroU8 are always ArchiveCopySafe
unsafe impl ArchiveCopySafe for () {}
unsafe impl<T: ?Sized> ArchiveCopySafe for PhantomData<T> {}
unsafe impl ArchiveCopySafe for PhantomPinned {}
unsafe impl ArchiveCopySafe for bool {}
unsafe impl ArchiveCopySafe for i8 {}
unsafe impl ArchiveCopySafe for u8 {}
unsafe impl ArchiveCopySafe for NonZeroI8 {}
unsafe impl ArchiveCopySafe for NonZeroU8 {}

// Multibyte integers are ArchiveCopySafe if the target matches the archived endianness
#[cfg(any(
    all(target_endian = "little", feature = "little_endian"),
    all(target_endian = "big", feature = "big_endian"),
))]
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
    unsafe impl ArchiveCopySafe for NonZeroI16 {}
    unsafe impl ArchiveCopySafe for NonZeroI32 {}
    unsafe impl ArchiveCopySafe for NonZeroI64 {}
    unsafe impl ArchiveCopySafe for NonZeroI128 {}
    unsafe impl ArchiveCopySafe for NonZeroU16 {}
    unsafe impl ArchiveCopySafe for NonZeroU32 {}
    unsafe impl ArchiveCopySafe for NonZeroU64 {}
    unsafe impl ArchiveCopySafe for NonZeroU128 {}
};

macro_rules! impl_tuple {
    () => {};
    (T, $($ts:ident,)*) => {
        unsafe impl<T: ArchiveCopySafe> ArchiveCopySafe for (T, $($ts,)*) {}

        impl_tuple!($($ts,)*);
    };
}

impl_tuple!(T, T, T, T, T, T, T, T, T, T, T,);

unsafe impl<T: ArchiveCopySafe, const N: usize> ArchiveCopySafe for [T; N] {}

/// Types that may be copy optimized.
///
/// By default, only [`ArchiveCopySafe`] types may be copy optimized. By enabling the `copy_unsafe`
/// feature, all types that are [`ArchiveCopy`] may be copy optimized.
#[cfg(not(feature = "copy_unsafe"))]
#[rustc_unsafe_specialization_marker]
pub trait ArchiveCopyOptimize: ArchiveCopySafe {}

#[cfg(not(feature = "copy_unsafe"))]
impl<T: ArchiveCopySafe> ArchiveCopyOptimize for T {}

/// Types that may be copy optimized.
///
/// By default, only [`ArchiveCopySafe`] types may be copy optimized. By enabling the `copy_unsafe`
/// feature, all types that are [`ArchiveCopy`] may be copy optimized.
#[cfg(feature = "copy_unsafe")]
#[rustc_unsafe_specialization_marker]
pub trait ArchiveCopyOptimize: ArchiveCopy {}

#[cfg(feature = "copy_unsafe")]
impl<T: ArchiveCopy> ArchiveCopyOptimize for T {}
