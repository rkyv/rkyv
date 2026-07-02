#[cfg(feature = "pointer_width_16")]
macro_rules! match_pointer_width {
    ($s16:ty, $s32:ty, $s64:ty $(,)?) => {
        $s16
    };
}

// If neither `pointer_width_16` nor `pointer_width_64` are enabled, then set
// the pointer width to 32.
#[cfg(not(any(feature = "pointer_width_16", feature = "pointer_width_64")))]
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

#[cfg(test)]
#[allow(unused)] // This macro is unused in some feature combinations
macro_rules! assert_source {
    ($e:expr, $p:pat $(,)?) => {
        assert_source!($e, $p, "left matches right");
    };
    ($e:expr, $p:pat, $msg:expr $(,)?) => {
        #[allow(unused)]
        let e = $e;
        #[cfg(all(debug_assertions, feature = "alloc"))]
        {
            let mut e = ::rancor::Error::inner(&e);
            while let Some(source) = ::core::error::Error::source(e) {
                e = source;
            }
            if let Some(e) = e.downcast_ref() {
                if !matches!(e, $p) {
                    panic!(
                        "assertion `{}` failed\nleft: {e:?}\nright: {}",
                        $msg,
                        ::core::stringify!($p),
                    )
                }
            } else {
                panic!(
                    "assertion `{}` failed\nleft: {e:?}\nright: {}",
                    $msg,
                    ::core::stringify!($p),
                );
            }
        }
    };
}
