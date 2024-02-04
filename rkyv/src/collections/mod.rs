//! Archived versions of standard library containers.

pub mod btree_map;
pub mod btree_set;
pub mod swiss_table;
// TODO: move these into a separate crate when indexmap adds rkyv support
// pub mod index_map;
// pub mod index_set;
pub mod util;

pub use self::btree_map::ArchivedBTreeMap;
// TODO: move these into a separate crate when indexmap adds rkyv support
// pub use self::index_map::ArchivedIndexMap;
// pub use self::index_set::ArchivedIndexSet;
