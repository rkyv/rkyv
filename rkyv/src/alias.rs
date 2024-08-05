use crate::{
    primitive::ArchivedIsize, rel_ptr, traits::ArchivePointee, Archive,
    ArchiveUnsized,
};

/// The default raw relative pointer.
///
/// This will use an archived [`FixedIsize`](crate::primitive::FixedIsize) to
/// hold the offset.
pub type RawRelPtr = rel_ptr::RawRelPtr<ArchivedIsize>;

/// The default relative pointer.
///
/// This will use an archived [`FixedIsize`](crate::primitive::FixedIsize) to
/// hold the offset.
pub type RelPtr<T> = rel_ptr::RelPtr<T, ArchivedIsize>;

/// Alias for the archived version of some [`Archive`] type.
///
/// This can be useful for reducing the lengths of type definitions.
pub type Archived<T> = <T as Archive>::Archived;

/// Alias for the resolver for some [`Archive`] type.
///
/// This can be useful for reducing the lengths of type definitions.
pub type Resolver<T> = <T as Archive>::Resolver;

/// Alias for the archived metadata for some [`ArchiveUnsized`] type.
///
/// This can be useful for reducing the lengths of type definitions.
pub type ArchivedMetadata<T> =
    <<T as ArchiveUnsized>::Archived as ArchivePointee>::ArchivedMetadata;
