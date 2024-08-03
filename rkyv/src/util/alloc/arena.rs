use crate::ser::allocator::Arena;

#[cfg(feature = "std")]
mod detail {
    use core::cell::Cell;

    use crate::ser::allocator::Arena;

    thread_local! {
        static THREAD_ARENA: Cell<Option<Arena>> = const { Cell::new(None) };
    }

    pub fn with_arena<T>(f: impl FnOnce(&mut Arena) -> T) -> T {
        THREAD_ARENA.with(|thread_arena| {
            let mut arena = thread_arena.take().unwrap_or_default();

            let result = f(&mut arena);
            let capacity = arena.shrink();

            if let Some(other) = thread_arena.take() {
                if other.capacity() > capacity {
                    arena = other;
                }
            }
            thread_arena.set(Some(arena));

            result
        })
    }

    #[inline]
    pub fn clear_arena() {
        THREAD_ARENA.take();
    }
}

#[cfg(not(feature = "std"))]
mod detail {
    use core::{
        ptr::{self, NonNull},
        sync::atomic::{AtomicPtr, Ordering},
    };

    use crate::ser::allocator::Arena;

    static GLOBAL_ARENA: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

    pub fn with_arena<T>(f: impl FnOnce(&mut Arena) -> T) -> T {
        let ptr = GLOBAL_ARENA.swap(ptr::null_mut(), Ordering::AcqRel);

        let mut arena = if let Some(raw) = NonNull::new(ptr) {
            unsafe { Arena::from_raw(raw) }
        } else {
            Arena::new()
        };

        let result = f(&mut arena);
        arena.shrink();

        let raw = arena.into_raw();

        let swap = GLOBAL_ARENA.compare_exchange(
            ptr::null_mut(),
            raw.as_ptr(),
            Ordering::AcqRel,
            Ordering::Relaxed,
        );
        if swap.is_err() {
            // Another arena was swapped in while we were executing `f`. We need
            // to free the current arena.
            unsafe {
                drop(Arena::from_raw(raw));
            }
        }

        result
    }

    #[inline]
    pub fn clear_arena() {
        let ptr = GLOBAL_ARENA.swap(ptr::null_mut(), Ordering::AcqRel);

        if let Some(raw) = NonNull::new(ptr) {
            unsafe {
                drop(Arena::from_raw(raw));
            }
        }
    }
}

/// Calls the given function with the builtin arena allocator.
///
/// When the `std` feature is enabled, the builtin arena allocator is a
/// thread-local variable, with one allocator per thread. Otherwise, it is a
/// global static and all threads share the same arena.
pub fn with_arena<T>(f: impl FnOnce(&mut Arena) -> T) -> T {
    detail::with_arena(f)
}

/// Clears the builtin arena allocator.
///
/// When the `std` feature is enabled, this only clears the allocator for the
/// current thread.
#[inline]
pub fn clear_arena() {
    detail::clear_arena()
}
