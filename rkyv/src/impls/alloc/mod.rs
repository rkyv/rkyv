use core::alloc::Layout;

use crate::alloc::alloc::dealloc;

/// Owns only an allocation, not a value being deserialized into it. The
/// unsized slice reader drops its initialized prefix when decoding fails.
pub(super) struct UninitializedAllocation {
    data_address: *mut u8,
    layout: Layout,
}

impl UninitializedAllocation {
    pub(super) fn new(data_address: *mut u8, layout: Layout) -> Self {
        Self {
            data_address,
            layout,
        }
    }

    pub(super) fn disarm(self) {
        core::mem::forget(self);
    }
}

impl Drop for UninitializedAllocation {
    fn drop(&mut self) {
        if self.layout.size() > 0 {
            // SAFETY: This guard receives the address and layout from the
            // matching allocation, and runs only before an owning value is
            // made from that allocation.
            unsafe { dealloc(self.data_address, self.layout) };
        }
    }
}

mod boxed;
mod collections;
mod ffi;
mod rc;
mod string;
mod vec;
mod with;

#[cfg(all(test, feature = "std"))]
mod ownership_tests;
