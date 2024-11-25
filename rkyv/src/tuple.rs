//! Archived versions of tuple types.

use crate::Portable;

macro_rules! impl_tuple {
    ($name:ident $n:tt, $($t:ident $u:ident $index:tt),* $(,)?) => {
        #[doc = concat!("An archived tuple with ", stringify!($n), " elements")]
        #[derive(
            Debug,
            Default,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            Portable,
        )]
        #[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
        #[rkyv(crate)]
        #[repr(C)]
        pub struct $name<$($t),*>($(pub $t),*);

        impl<$($t,)* $($u),*> PartialEq<($($u,)*)> for $name<$($t),*>
        where
            $($t: PartialEq<$u>,)*
        {
            fn eq(&self, other: &($($u,)*)) -> bool {
                $(self.$index == other.$index)&&*
            }
        }
    };
}

impl_tuple!(ArchivedTuple1 1, T0 U0 0);
impl_tuple!(ArchivedTuple2 2, T0 U0 0, T1 U1 1);
impl_tuple!(ArchivedTuple3 3, T0 U0 0, T1 U1 1, T2 U2 2);
impl_tuple!(ArchivedTuple4 4, T0 U0 0, T1 U1 1, T2 U2 2, T3 U3 3);
impl_tuple!(ArchivedTuple5 5, T0 U0 0, T1 U1 1, T2 U2 2, T3 U3 3, T4 U4 4);
impl_tuple!(
    ArchivedTuple6 6, T0 U0 0, T1 U1 1, T2 U2 2, T3 U3 3, T4 U4 4, T5 U5 5
);
impl_tuple!(
    ArchivedTuple7 7,
    T0 U0 0,
    T1 U1 1,
    T2 U2 2,
    T3 U3 3,
    T4 U4 4,
    T5 U5 5,
    T6 U6 6,
);
impl_tuple!(
    ArchivedTuple8 8,
    T0 U0 0,
    T1 U1 1,
    T2 U2 2,
    T3 U3 3,
    T4 U4 4,
    T5 U5 5,
    T6 U6 6,
    T7 U7 7,
);
impl_tuple!(
    ArchivedTuple9 9,
    T0 U0 0,
    T1 U1 1,
    T2 U2 2,
    T3 U3 3,
    T4 U4 4,
    T5 U5 5,
    T6 U6 6,
    T7 U7 7,
    T8 U8 8,
);
impl_tuple!(
    ArchivedTuple10 10,
    T0 U0 0,
    T1 U1 1,
    T2 U2 2,
    T3 U3 3,
    T4 U4 4,
    T5 U5 5,
    T6 U6 6,
    T7 U7 7,
    T8 U8 8,
    T9 U9 9
);
impl_tuple!(
    ArchivedTuple11 11,
    T0 U0 0,
    T1 U1 1,
    T2 U2 2,
    T3 U3 3,
    T4 U4 4,
    T5 U5 5,
    T6 U6 6,
    T7 U7 7,
    T8 U8 8,
    T9 U9 9,
    T10 U10 10,
);
impl_tuple!(
    ArchivedTuple12 12,
    T0 U0 0,
    T1 U1 1,
    T2 U2 2,
    T3 U3 3,
    T4 U4 4,
    T5 U5 5,
    T6 U6 6,
    T7 U7 7,
    T8 U8 8,
    T9 U9 9,
    T10 U10 10,
    T11 U11 11,
);
impl_tuple!(
    ArchivedTuple13 13,
    T0 U0 0,
    T1 U1 1,
    T2 U2 2,
    T3 U3 3,
    T4 U4 4,
    T5 U5 5,
    T6 U6 6,
    T7 U7 7,
    T8 U8 8,
    T9 U9 9,
    T10 U10 10,
    T11 U11 11,
    T12 U12 12,
);

#[cfg(test)]
mod tests {
    use crate::tuple::ArchivedTuple3;

    #[test]
    fn partial_eq() {
        assert_eq!(ArchivedTuple3(1, 2, 3), (1, 2, 3));
    }
}
