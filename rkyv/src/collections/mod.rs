//! Archived versions of standard library containers.

mod btree;
pub use btree::{map as btree_map, set as btree_set};
pub mod swiss_table;
pub mod util;
