use crate::Archive;

macro_rules! impl_primitive {
    ($type:ty) => (
        impl Archive for $type
        where
            $type: Copy
        {
            type Archived = $type;

            fn archive(&self) -> Self::Archived {
                *self
            }

            fn unarchive(archived: &Self::Archived) -> Self {
                *archived
            }
        }
    )
}

impl_primitive!(());
impl_primitive!(bool);
impl_primitive!(char);
impl_primitive!(f32);
impl_primitive!(f64);
impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(i128);
impl_primitive!(u8);
impl_primitive!(u16);
impl_primitive!(u32);
impl_primitive!(u64);
impl_primitive!(u128);

macro_rules! peel_tuple {
    ($first:ident, $($rest:ident,)*) => (impl_tuple! { $($rest,)* })
}

macro_rules! impl_tuple {
    () => ();
    ($($type:ident,)+) => (
        #[allow(non_snake_case)]
        impl <$($type: Archive),+> Archive for ($($type,)+) {
            type Archived = ($($type::Archived,)+);

            fn archive(&self) -> Self::Archived {
                let ($(ref $type,)+) = *self;
                ($($type.archive(),)+)
            }

            fn unarchive(archived: &Self::Archived) -> Self {
                let ($(ref $type,)+) = *archived;
                ($(<$type as Archive>::unarchive($type),)+)
            }
        }

        peel_tuple! { $($type,)+ }
    );
}

impl_tuple! { T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, }

#[cfg(not(feature = "const_generics"))]
macro_rules! impl_array {
    {$len:expr, $n:literal $($ns:literal)*} => {
        impl<T: Archive> Archive for [T; $len] {
            type Archived = [T::Archived; $len];

            fn archive(&self) -> Self::Archived {
                [self[$n].archive(), $(self[$ns].archive(),)*]
            }

            fn unarchive(archived: &Self::Archived) -> Self {
                [T::unarchive(&archived[$n]), $(T::unarchive(&archived[$ns]),)*]
            }
        }

        impl_array! { ($len - 1), $($ns)* }
    };
    {$len:expr,} => {
        impl<T: Archive> Archive for [T; $len] {
            type Archived = [T::Archived; $len];

            fn archive(&self) -> Self::Archived {
                []
            }

            fn unarchive(_: &Self::Archived) -> Self {
                []
            }
        }
    };
}

impl_array! { 32, 31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13 12 11 10 9 8 7 6 5 4 3 2 1 0 }

#[cfg(feature = "const_generics")]
impl<T: Archive, const N: usize> Archive for [T; N] {
    type Archived = [T::Archived; N];

    fn archive(&self) -> Self::Archived {
        let mut result = core::mem::MaybeUninit::<[T::Archived; N]>::uninit();
        for i in 0..N {
            unsafe {
                core::ptr::write(
                    &mut (*result.as_mut_ptr())[i] as *mut T::Archived,
                    self[i].archive()
                );
            }
        }
        unsafe {
            result.assume_init()
        }
    }

    fn unarchive(archived: &Self::Archived) -> Self {
        let mut result = core::mem::MaybeUninit::<Self>::uninit();
        for i in 0..N {
            unsafe {
                core::ptr::write(
                    &mut (*result.as_mut_ptr())[i] as *mut T,
                    T::unarchive(&archived[i])
                )
            }
        }
        unsafe {
            result.assume_init()
        }
    }
}

// TODO: str, slice