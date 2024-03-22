use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

/// A value that can be initialized once lazily.
pub struct LazyStatic<T> {
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> LazyStatic<T> {
    const UNINITIALIZED: u8 = 0;
    const INITIALIZING: u8 = 1;
    const INITIALIZED: u8 = 2;

    /// Creates a new uninitialized `LazyStatic`.
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(Self::UNINITIALIZED),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Initializes the `LazyStatic`, failing if it is initializing or already
    /// initialized.
    ///
    /// If initialization succeeds, this returns a reference to the initialized
    /// value. If it fails, it returns the value that initialization was
    /// attempted with.
    pub fn init(&self, value: T) -> Result<&T, T> {
        let acquire = self.state.compare_exchange(
            Self::UNINITIALIZED,
            Self::INITIALIZING,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );

        if acquire.is_err() {
            Err(value)
        } else {
            // SAFETY: We acquired exclusive access to `value` by atomically
            // swapping `state` from `UNINITIALIZED` to `INITIALIZING`.
            let mu = unsafe { &mut *self.value.get() };
            mu.write(value);

            self.state.store(Self::INITIALIZED, Ordering::Release);

            // SAFETY: We just initialized this `LazyStatic`.
            Ok(unsafe { self.get_unchecked() })
        }
    }

    unsafe fn get_unchecked(&self) -> &T {
        // SAFETY: The caller has guaranteed that state is `INITIALIZED`, so
        // `value` is not currently and never will be exclusively aliased again.
        let mu = unsafe { &*self.value.get() };
        // SAFETY: The caller has guaranteed that state is `INITIALIZED`, so
        // `value` is initialized.
        unsafe { mu.assume_init_ref() }
    }

    /// Returns the value of the `LazyStatic`, or `None` if it is not
    /// initialized.
    pub fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) == Self::INITIALIZED {
            // SAFETY: We checked that this `LazyStatic` is initialized.
            Some(unsafe { self.get_unchecked() })
        } else {
            None
        }
    }
}

impl<T> Default for LazyStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: `LazyStatic` ensures that access to the underlying value is safe with
// multiple threads, so `LazyStatic<T>` is `Sync` as long as `T` is also `Sync`.
unsafe impl<T: Sync> Sync for LazyStatic<T> {}
