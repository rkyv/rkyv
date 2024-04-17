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
