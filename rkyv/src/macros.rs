#[cfg(all(feature = "std", feature = "specialization"))]
macro_rules! default {
    ($($fn:tt)*) => { default $($fn)* };
}

#[cfg(all(feature = "std", not(feature = "specialization")))]
macro_rules! default {
    ($($fn:tt)*) => { $($fn)* };
}

/// Maps a mutable `MaybeUninit` struct reference to a mutable `MaybeUninit` field reference.
///
/// This is primarily used to succintly resolve the fields of structs into the output for
/// [`resolve()`](crate::Archive::resolve()).
///
/// # Example
///
/// ```
/// use rkyv::project_struct;
/// use core::mem::MaybeUninit;
///
/// struct Test {
///     a: i32,
///     b: u8,
/// }
///
/// let result = unsafe {
///     let mut result = MaybeUninit::<Test>::uninit();
///     let out = &mut result;
///     project_struct!(out: Test => a: i32).as_mut_ptr().write(42);
///     project_struct!(out: Test => b: u8).as_mut_ptr().write(10);
///     result.assume_init()
/// };
///
/// assert_eq!(result.a, 42);
/// assert_eq!(result.b, 10);
/// ```
#[macro_export]
macro_rules! project_struct {
    ($struct:ident: $ty:path => $field:tt) => {
        #[allow(unused_unsafe)]
        (unsafe {
            &mut *($struct as &mut MaybeUninit<$ty>)
                .as_mut_ptr()
                .cast::<u8>()
                .add($crate::offset_of!($ty, $field))
                .cast()
        })
    };
    ($struct:ident: $struct_ty:path => $field:tt: $field_ty:path) => {
        #[allow(unused_unsafe)]
        (unsafe {
            &mut *($struct as &mut MaybeUninit<$struct_ty>)
                .as_mut_ptr()
                .cast::<u8>()
                .add($crate::offset_of!($struct_ty, $field))
                .cast::<MaybeUninit<$field_ty>>()
        })
    };
}

/// Maps a mutable `MaybeUninit` tuple reference to a mutable `MaybeUninit` index reference.
///
/// This is primarily used to succintly resolve the fields of tuples into the output for
/// [`resolve()`](crate::Archive::resolve()).
///
/// # Example
///
/// ```
/// use rkyv::project_tuple;
/// use core::mem::MaybeUninit;
///
/// let result = unsafe {
///     let mut result = MaybeUninit::<(i32, u8)>::uninit();
///     let out = &mut result;
///     project_tuple!(out: (i32, u8) => 0: i32).as_mut_ptr().write(42);
///     project_tuple!(out: (i32, u8) => 1: u8).as_mut_ptr().write(10);
///     result.assume_init()
/// };
///
/// assert_eq!(result.0, 42);
/// assert_eq!(result.1, 10);
/// ```
#[macro_export]
macro_rules! project_tuple {
    ($tuple:ident: $ty:ty => $index:tt) => {
        (unsafe {
            &mut *($tuple as &mut MaybeUninit<$ty>)
                .as_mut_ptr()
                .cast::<u8>()
                .add(memoffset::offset_of_tuple!($ty, $index))
                .cast()
        })
    };
    ($tuple:ident: $tuple_ty:ty => $index:tt: $index_ty:path) => {
        (unsafe {
            &mut *($tuple as &mut MaybeUninit<$tuple_ty>)
                .as_mut_ptr()
                .cast::<u8>()
                .add(memoffset::offset_of_tuple!($tuple_ty, $index))
                .cast::<MaybeUninit<$index_ty>>()
        })
    };
}

macro_rules! resolve_struct {
    ($out:ident = |$pos:ident, $resolver:ident| -> $out_ty:path { $($field:ident: $field_expr:expr),* }) => {
        resolve_struct!($out = |$pos, $resolver| -> $out_ty { $($field: $field_expr,)* });
    };
    ($out:ident = |$pos:ident, $resolver:ident| -> $out_ty:path { $($field:ident: $field_expr:expr,)* }) => {
        $(
            unsafe {
                let field_offset = memoffset::offset_of!($out_ty, $field);
                $field_expr.resolve(
                    $pos + field_offset,
                    $resolver.$field,
                    &mut *$out.as_mut_ptr().cast::<u8>().add(field_offset).cast()
                );
            }
        )*
    };
}
