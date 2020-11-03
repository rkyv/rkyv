use core::ops::Deref;
use crate::{
    Archive,
    Archived,
    Identity,
    rel_ptr,
    RelativePointer,
    Resolve,
    Resolver,
    Write,
};

macro_rules! impl_primitive {
    ($type:ty) => (
        impl Archive for $type
        where
            $type: Copy
        {
            type Resolver = Identity<Self>;

            fn archive<W: Write + ?Sized>(&self, _writer: &mut W) -> Result<Self::Resolver, W::Error> {
                Ok(Identity::new())
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
    ($type:ident $value:tt, $($type_rest:ident $value_rest:tt,)*) => { impl_tuple! { $($type_rest $value_rest,)* } };
}

macro_rules! impl_tuple {
    () => ();
    ($($type:ident $index:tt,)+) => {
        #[allow(non_snake_case)]
        impl<$($type: Archive),+> Resolve<($($type,)+)> for ($(Resolver<$type>,)+) {
            type Archived = ($(Archived<$type>,)+);

            fn resolve(self, pos: usize, value: &($($type,)+)) -> Self::Archived {
                let rev = ($(self.$index.resolve(pos + memoffset::offset_of_tuple!(Self::Archived, $index), &value.$index),)+);
                ($(rev.$index,)+)
            }
        }

        #[allow(non_snake_case)]
        impl<$($type: Archive),+> Archive for ($($type,)+) {
            type Resolver = ($(Resolver<$type>,)+);

            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                let rev = ($(self.$index.archive(writer)?,)+);
                Ok(($(rev.$index,)+))
            }
        }

        peel_tuple! { $($type $index,)+ }
    };
}

impl_tuple! { T11 11, T10 10, T9 9, T8 8, T7 7, T6 6, T5 5, T4 4, T3 3, T2 2, T1 1, T0 0, }

#[cfg(not(feature = "const_generics"))]
macro_rules! impl_array {
    () => ();
    ($len:literal, $($rest:literal,)*) => {
        impl<T: Archive> Resolve<[T; $len]> for [Resolver<T>; $len] {
            type Archived = [Archived<T>; $len];

            fn resolve(self, pos: usize, value: &[T; $len]) -> Self::Archived {
                let mut resolvers = core::mem::MaybeUninit::new(self);
                let resolvers_ptr = resolvers.as_mut_ptr().cast::<Resolver<T>>();
                let mut result = core::mem::MaybeUninit::<Self::Archived>::uninit();
                let result_ptr = result.as_mut_ptr().cast::<Archived<T>>();
                for i in 0..$len {
                    unsafe {
                        result_ptr.add(i).write(resolvers_ptr.add(i).read().resolve(pos + i * core::mem::size_of::<T>(), &value[i]));
                    }
                }
                unsafe {
                    result.assume_init()
                }
            }
        }

        impl<T: Archive> Archive for [T; $len] {
            type Resolver = [Resolver<T>; $len];

            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                let mut result = core::mem::MaybeUninit::<Self::Resolver>::uninit();
                let result_ptr = result.as_mut_ptr().cast::<Resolver<T>>();
                for i in 0..$len {
                    unsafe {
                        result_ptr.add(i).write(self[i].archive(writer)?);
                    }
                }
                unsafe {
                    Ok(result.assume_init())
                }
            }
        }

        impl_array! { $($rest,)* }
    };
}

#[cfg(not(feature = "const_generics"))]
impl_array! { 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, }

#[cfg(feature = "const_generics")]
impl<T: Archive, const N: usize> Resolve<[T; N]> for [Resolver<T>; N] {
    type Archived = [Archived<T>; N];

    fn resolve(self, pos: usize, value: &[T; N]) -> Self::Archived {
        let mut resolvers = core::mem::MaybeUninit::new(self);
        let resolvers_ptr = resolvers.as_mut_ptr().cast::<Resolver<T>>();
        let mut result = core::mem::MaybeUninit::<Self::Archived>::uninit();
        let result_ptr = result.as_mut_ptr().cast::<Archived<T>>();
        for i in 0..N {
            unsafe {
                result_ptr.add(i).write(resolvers_ptr.add(i).read().resolve(pos + i * core::mem::size_of::<T>(), &value[i]));
            }
        }
        unsafe {
            result.assume_init()
        }
    }
}

#[cfg(feature = "const_generics")]
impl<T: Archive, const N: usize> Archive for [T; N] {
    type Resolver = [Resolver<T>; N];

    fn archive<W: Write>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        let mut result = core::mem::MaybeUninit::<[Resolver<T>; N]>::uninit();
        let result_ptr = result.as_mut_ptr().cast::<Resolver<T>>();
        for i in 0..N {
            unsafe {
                result_ptr.add(i).write(self[i].archive(writer)?);
            }
        }
        unsafe {
            Ok(result.assume_init())
        }
    }
}

pub struct ArchivedStr {
    ptr: RelativePointer<u8>,
    len: u32,
}

impl ArchivedStr {
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.as_ptr(), self.len as usize)
        }
    }
}

impl Deref for ArchivedStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe {
            core::str::from_utf8_unchecked(self.as_bytes())
        }
    }
}

impl Resolve<str> for usize {
    type Archived = ArchivedStr;

    fn resolve(self, pos: usize, value: &str) -> Self::Archived {
        Self::Archived {
            ptr: rel_ptr!(pos, Self::Archived, ptr, self),
            len: value.len() as u32,
        }
    }
}

impl Archive for str {
    type Resolver = usize;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        let result = writer.pos();
        writer.write(self.as_bytes())?;
        Ok(result)
    }
}
