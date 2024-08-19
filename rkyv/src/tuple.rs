//! Archived versions of tuple types.

use crate::{traits::Freeze, Portable};

macro_rules! impl_tuple {
    ($name:ident, $n:tt, $($type:ident $index:tt),*) => {
        #[doc = concat!("An archived tuple with ", stringify!($n), " elements")]
        #[derive(Debug, Freeze, Portable)]
        #[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
        #[repr(C)]
        #[rkyv(crate)]
        pub struct $name<$($type),*>($(pub $type),*);
    };
}

impl_tuple!(ArchivedTuple1, 1, T0 0);
impl_tuple!(ArchivedTuple2, 2, T0 0, T1 1);
impl_tuple!(ArchivedTuple3, 3, T0 0, T1 1, T2 2);
impl_tuple!(ArchivedTuple4, 4, T0 0, T1 1, T2 2, T3 3);
impl_tuple!(ArchivedTuple5, 5, T0 0, T1 1, T2 2, T3 3, T4 4);
impl_tuple!(ArchivedTuple6, 6, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5);
impl_tuple!(ArchivedTuple7, 7, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6);
impl_tuple!(ArchivedTuple8, 8, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7);
impl_tuple!(
    ArchivedTuple9, 9, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8
);
impl_tuple!(
    ArchivedTuple10, 10, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8,
    T9 9
);
impl_tuple!(
    ArchivedTuple11, 11, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8,
    T9 9, T10 10
);
impl_tuple!(
    ArchivedTuple12, 12, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8,
    T9 9, T10 10, T11 11
);
impl_tuple!(
    ArchivedTuple13, 13, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8,
    T9 9, T10 10, T11 11, T12 12
);
