use core::{alloc::Layout, ptr::NonNull};

pub fn dangling(layout: &Layout) -> NonNull<u8> {
    #[cfg(miri)]
    {
        layout.dangling()
    }
    #[cfg(not(miri))]
    unsafe {
        NonNull::new_unchecked(layout.align() as *mut u8)
    }
}
