//! SwissTable-based implementation for archived hash map and hash set.

pub mod map;
pub mod set;
pub mod table;

pub use map::{ArchivedHashMap, HashMapResolver};
pub use set::{ArchivedHashSet, HashSetResolver};
pub use table::{ArchivedHashTable, HashTableResolver};
