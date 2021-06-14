//! Archived containers to use with or without the standard library.

pub mod hash_map;
pub mod hash_set;

pub use self::hash_map::ArchivedHashMap;
pub use self::hash_set::ArchivedHashSet;
