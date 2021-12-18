use ::core::fmt;
use crate::{
    de::deserializers::SharedDeserializeMap,
    validation::validators::{CheckTypeError, DefaultValidator},
    check_archived_root,
    Archive,
    Deserialize,
    Fallible,
};
use ::bytecheck::CheckBytes;

/// Errors that can occur while deserializing from bytes.
#[derive(Debug)]
pub enum FromBytesError<C, D> {
    /// A validation error occurred.
    CheckBytesError(C),
    /// A deserialization error occurred.
    DeserializeError(D),
}

impl<C: fmt::Display, D: fmt::Display> fmt::Display for FromBytesError<C, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CheckBytesError(e) => write!(f, "{}", e),
            Self::DeserializeError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use ::std::error::Error;

    impl<C: Error + 'static, D: Error + 'static> Error for FromBytesError<C, D> {
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
/// This function is only available with the `alloc` feature because it uses a general-purpose
/// deserializer. In no-alloc and high-performance environments, the deserializer should be
/// customized for the specific situation.
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
pub fn from_bytes<'a, T>(bytes: &'a [u8]) -> Result<T, FromBytesError<CheckTypeError<T::Archived, DefaultValidator<'a>>, <SharedDeserializeMap as Fallible>::Error>>
where
    T: Archive,
    T::Archived: 'a + CheckBytes<DefaultValidator<'a>> + Deserialize<T, SharedDeserializeMap>
{
    check_archived_root::<'a, T>(bytes)
        .map_err(FromBytesError::CheckBytesError)?
        .deserialize(&mut SharedDeserializeMap::default())
        .map_err(FromBytesError::DeserializeError)
}
