#[cfg(feature = "copy")]
macro_rules! default {
    ($($fn:tt)*) => { default $($fn)* };
}

#[cfg(not(feature = "copy"))]
macro_rules! default {
    ($($fn:tt)*) => { $($fn)* };
}

#[macro_export]
macro_rules! out_field {
    ($out:ident.$field:tt) => {
        {
            fn as_uninit<'a, T, U>(_: &'a mut ::core::mem::MaybeUninit<T>, ptr: *mut U) -> &'a mut ::core::mem::MaybeUninit<U> {
                unsafe {
                    &mut *ptr.cast()
                }
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
        }
    };
}
