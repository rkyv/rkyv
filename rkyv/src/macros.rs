#[cfg(feature = "copy")]
macro_rules! default {
    ($($fn:tt)*) => { default $($fn)* };
}

#[cfg(not(feature = "copy"))]
macro_rules! default {
    ($($fn:tt)*) => { $($fn)* };
}

/// Returns a tuple of the field offset and a mutable `MaybeUninit` to the given field of the given
/// `MaybeUninit` struct.
///
/// # Examples
/// ```
/// use core::mem::MaybeUninit;
/// use rkyv::out_field;
///
/// struct Example {
///     a: i32,
///     b: bool,
/// }
///
/// let mut result = MaybeUninit::<Example>::zeroed();
/// let out = &mut result;
///
/// let (a_off, a) = out_field!(out.a);
/// unsafe { a.as_mut_ptr().write(42); }
/// let (b_off, b) = out_field!(out.b);
/// unsafe { b.as_mut_ptr().write(true); }
///
/// let result = unsafe { result.assume_init() };
/// assert_eq!(result.a, 42);
/// assert_eq!(result.b, true);
/// ```
#[macro_export]
macro_rules! out_field {
    ($out:ident.$field:tt) => {{
        #[inline(always)]
        fn as_uninit<'a, T, U>(
            _: &'a mut ::core::mem::MaybeUninit<T>,
            ptr: *mut U,
        ) -> &'a mut ::core::mem::MaybeUninit<U> {
            unsafe { &mut *ptr.cast() }
        }
        let out_ptr = $out.as_mut_ptr();
        #[allow(unused_unsafe)]
        unsafe {
            let field_out = ::core::ptr::addr_of_mut!((*out_ptr).$field);
            (
                field_out.cast::<u8>().offset_from(out_ptr.cast::<u8>()) as usize,
                as_uninit($out, field_out),
            )
        }
    }};
}

/// Returns the unarchived value of the given archived primitive.
///
/// This macro is not needed for most use cases. Its primary purpose is to simultaneously:
/// - Convert values from (potentially) different archived primitives to their native counterparts
/// - Allow transformation in `const` contexts
/// - Prevent linter warnings from unused `into()` calls
///
/// Users should feel free to use the more ergonomic `into()` where appropriate.
#[macro_export]
macro_rules! from_archived {
    ($expr:expr) => {{
        #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
        {
            $expr
        }
        #[cfg(any(feature = "archive_le", feature = "archive_be"))]
        {
            ($expr).value()
        }
    }};
}

/// Returns the archived value of the given archived primitive.
///
/// This macro is not needed for most use cases. Its primary purpose is to simultaneously:
/// - Convert values from (potentially) different primitives to their archived counterparts
/// - Allow transformation in `const` contexts
/// - Prevent linter warnings from unused `into()` calls
///
/// Users should feel free to use the more ergonomic `into()` where appropriate.
#[macro_export]
macro_rules! to_archived {
    ($expr:expr) => {{
        #[cfg(not(any(feature = "archive_le", feature = "archive_be")))]
        {
            $expr
        }
        #[cfg(feature = "archive_le")]
        {
            $crate::impls::core::primitive::NativeEndian { value: $expr }.to_le()
        }
        #[cfg(feature = "archive_be")]
        {
            $crate::impls::core::primitive::NativeEndian { value: $expr }.to_be()
        }
    }};
}

#[cfg(feature = "size_16")]
macro_rules! pick_size_type {
    ($s16:ty, $s32:ty, $s64:ty) => { $s16 }
}

#[cfg(feature = "size_32")]
macro_rules! pick_size_type {
    ($s16:ty, $s32:ty, $s64:ty) => { $s32 }
}

#[cfg(feature = "size_64")]
macro_rules! pick_size_type {
    ($s16:ty, $s32:ty, $s64:ty) => { $s64 }
}
