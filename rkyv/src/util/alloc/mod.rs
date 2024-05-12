mod aligned_vec;

use rancor::Strategy;

pub use self::aligned_vec::*;
use crate::{
    access_unchecked,
    de::pooling::Pool,
    deserialize,
    ser::{allocator::Arena, sharing::Share, DefaultSerializer, Serializer},
    util::serialize_into,
    Archive, Deserialize, Serialize,
};

#[cfg(feature = "std")]
mod arena {
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

    pub fn clear_arena() {
        THREAD_ARENA.take();
    }
}

#[cfg(not(feature = "std"))]
mod arena {
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
#[inline]
pub fn with_arena<T>(f: impl FnOnce(&mut Arena) -> T) -> T {
    arena::with_arena(f)
}

/// Clears the global arena allocator.
///
/// When the `std` feature is enabled, this only clears the allocator for the
/// current thread.
#[inline]
pub fn clear_arena() {
    arena::clear_arena()
}

/// Serializes the given value and returns the resulting bytes.
///
/// The const generic parameter `N` specifies the number of bytes to
/// pre-allocate as scratch space. Choosing a good default value for your data
/// can be difficult without any data, so consider using an
/// [`AllocationTracker`](crate::ser::allocator::AllocationTracker) to determine
/// how much scratch space is typically used.
///
/// This function is only available with the `alloc` feature because it uses a
/// general-purpose serializer. In no-alloc and high-performance environments,
/// the serializer should be customized for the specific situation.
///
/// # Examples
/// ```
/// use rkyv::rancor::Error;
///
/// let value = vec![1, 2, 3, 4];
///
/// let bytes =
///     rkyv::to_bytes::<Error>(&value).expect("failed to serialize vec");
/// // SAFETY:
/// // - The byte slice represents an archived object
/// // - The root of the object is stored at the end of the slice
/// let deserialized = unsafe {
///     rkyv::from_bytes_unchecked::<Vec<i32>, Error>(&bytes)
///         .expect("failed to deserialize vec")
/// };
///
/// assert_eq!(deserialized, value);
/// ```
#[inline]
pub fn to_bytes<E: rancor::Source>(
    value: &impl for<'a> Serialize<DefaultSerializer<'a, E>>,
) -> Result<AlignedVec, E> {
    with_arena(|arena| {
        Ok(serialize_into(
            value,
            Serializer::new(AlignedVec::new(), arena.acquire(), Share::new()),
        )?
        .into_writer())
    })
}

/// Deserializes a value from the given bytes.
///
/// This function is only available with the `alloc` feature because it uses a
/// general-purpose deserializer. In no-alloc and high-performance environments,
/// the deserializer should be customized for the specific situation.
///
/// # Safety
///
/// - The byte slice must represent an archived object.
/// - The root of the object must be stored at the end of the slice (this is the
///   default behavior).
///
/// # Examples
/// ```
/// use rkyv::rancor::Error;
///
/// let value = vec![1, 2, 3, 4];
///
/// let bytes =
///     rkyv::to_bytes::<Error>(&value).expect("failed to serialize vec");
/// // SAFETY:
/// // - The byte slice represents an archived object
/// // - The root of the object is stored at the end of the slice
/// let deserialized = unsafe {
///     rkyv::from_bytes_unchecked::<Vec<i32>, Error>(&bytes)
///         .expect("failed to deserialize vec")
/// };
///
/// assert_eq!(deserialized, value);
/// ```
#[inline]
pub unsafe fn from_bytes_unchecked<T, E>(bytes: &[u8]) -> Result<T, E>
where
    T: Archive,
    T::Archived: Deserialize<T, Strategy<Pool, E>>,
{
    // SAFETY: The caller has guaranteed that a valid `T` is located at the root
    // position in the byte slice.
    let archived = unsafe { access_unchecked::<T::Archived>(bytes) };
    deserialize(archived, &mut Pool::new())
}
