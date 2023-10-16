use core::fmt;

use crate::{
    bytecheck::{CheckBytes, rancor::Error},
    check_archived_root,
    de::deserializers::SharedDeserializeMap,
    validation::validators::DefaultValidator,
    Archive, Deserialize,
};

/// Errors that can occur while deserializing from bytes.
#[derive(Debug)]
pub enum CheckDeserializeError<C, D> {
    /// A validation error occurred.
    CheckBytesError(C),
    /// A deserialization error occurred.
    DeserializeError(D),
}

impl<C: fmt::Display, D: fmt::Display> fmt::Display
    for CheckDeserializeError<C, D>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CheckBytesError(e) => write!(f, "{}", e),
            Self::DeserializeError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl<C: Error + 'static, D: Error + 'static> Error
        for CheckDeserializeError<C, D>
    {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                Self::CheckBytesError(e) => Some(e as &dyn Error),
                Self::DeserializeError(e) => Some(e as &dyn Error),
            }
        }
    }
};

/// Checks and deserializes a value from the given bytes.
///
/// This function is only available with the `alloc` and `validation` features because it uses a
/// general-purpose deserializer and performs validation on the data before deserializing. In
/// no-alloc and high-performance environments, the deserializer should be customized for the
/// specific situation.
///
/// # Examples
/// ```
/// let value = vec![1, 2, 3, 4];
///
/// let bytes = rkyv::to_bytes::<_, 1024>(&value).expect("failed to serialize vec");
/// let deserialized = rkyv::from_bytes::<Vec<i32>>(&bytes).expect("failed to deserialize vec");
///
/// assert_eq!(deserialized, value);
/// ```
#[inline]
pub fn from_bytes<T, E>(bytes: &[u8]) -> Result<T, E>
where
    T: Archive,
    T::Archived: CheckBytes<DefaultValidator, E> + Deserialize<T, SharedDeserializeMap, E>,
    E: Error,
{
    check_archived_root::<T, E>(bytes)?.deserialize(&mut SharedDeserializeMap::default())
}
