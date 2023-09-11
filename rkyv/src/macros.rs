#[cfg(feature = "copy")]
macro_rules! default {
    (#[inline] $($fn:tt)*) => { #[inline] default $($fn)* };
    ($($fn:tt)*) => { default $($fn)* };
}

#[cfg(not(feature = "copy"))]
macro_rules! default {
    (#[inline] $($fn:tt)*) => { #[inline] $($fn)* };
    ($($fn:tt)*) => { $($fn)* };
}

/// Returns a tuple of `(field_pos, field_out)`, where `field_pos` is the "position",
/// i.e. offset in bytes, of the field relative to the base address of the struct and `field_out`
/// is a `*mut` that points to the field directly.
///
/// This is essentially a convenience wrapper around [`core::ptr::addr_of_mut!`] that also
/// gives back the relative offset of the field, as these are often needed together. You will often
/// see the return values named `(fp, fo)` in internal use of this macro, which stand for `field_pos`
/// and `field_out` respectively as discussed above.
///
/// # Example
///
/// ```
/// use core::mem::MaybeUninit;
/// use rkyv::out_field;
///
/// // The macro works on repr(Rust) structs as well, but for the purposes of asserting
/// // an exact guaranteed position of each field in this example, we'll use repr(C)
/// #[repr(C)]
/// struct Example {
///     a: i32,
///     b: bool,
/// }
///
/// let mut result = MaybeUninit::<Example>::zeroed();
/// let out = result.as_mut_ptr();
///
/// let (a_pos, a_out) = out_field!(out.a);
/// assert_eq!(a_pos, 0); // guaranteed by repr(C) layout, repr(Rust) has free reign
/// unsafe { a_out.write(42); }
///
/// let (b_pos, b_out) = out_field!(out.b);
/// assert_eq!(b_pos, 4); // guaranteed by repr(C) layout, repr(Rust) has free reign
/// unsafe { b_out.write(true); }
///
/// let result = unsafe { result.assume_init() };
/// assert_eq!(result.a, 42);
/// assert_eq!(result.b, true);
/// ```
#[macro_export]
macro_rules! out_field {
    ($out:ident.$field:tt) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let fo = ::core::ptr::addr_of_mut!((*$out).$field);
            (fo.cast::<u8>().offset_from($out.cast::<u8>()) as usize, fo)
        }
    }};
}

#[cfg(feature = "pointer_width_16")]
macro_rules! match_pointer_width {
    ($s16:ty, $s32:ty, $s64:ty $(,)?) => {
        $s16
    };
}

#[cfg(feature = "pointer_width_32")]
macro_rules! match_pointer_width {
    ($s16:ty, $s32:ty, $s64:ty $(,)?) => {
        $s32
    };
}

#[cfg(feature = "pointer_width_64")]
macro_rules! match_pointer_width {
    ($s16:ty, $s32:ty, $s64:ty $(,)?) => {
        $s64
    };
}
