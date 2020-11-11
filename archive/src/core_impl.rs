use core::{
    borrow::Borrow,
    fmt,
    hash::{
        Hash,
        Hasher,
    },
    ops::Deref,
};
use crate::{
    Archive,
    ArchiveRef,
    ArchiveSelf,
    RelPtr,
    Resolve,
    SelfResolver,
    Write,
};

macro_rules! impl_primitive {
    ($type:ty) => (
        unsafe impl ArchiveSelf for $type {}

        impl Archive for $type
        where
            $type: Copy
        {
            type Archived = Self;
            type Resolver = SelfResolver;

            fn archive<W: Write + ?Sized>(&self, _writer: &mut W) -> Result<Self::Resolver, W::Error> {
                Ok(SelfResolver)
            }
        }
    )
}

impl_primitive!(());
impl_primitive!(bool);
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
impl_primitive!(f32);
impl_primitive!(f64);
impl_primitive!(char);

macro_rules! peel_tuple {
    ($type:ident $value:tt, $($type_rest:ident $value_rest:tt,)*) => { impl_tuple! { $($type_rest $value_rest,)* } };
}

macro_rules! impl_tuple {
    () => ();
    ($($type:ident $index:tt,)+) => {
        unsafe impl<$($type: ArchiveSelf),+> ArchiveSelf for ($($type,)+) {}

        #[allow(non_snake_case)]
        impl<$($type: Archive),+> Resolve<($($type,)+)> for ($($type::Resolver,)+) {
            type Archived = ($($type::Archived,)+);

            fn resolve(self, pos: usize, value: &($($type,)+)) -> Self::Archived {
                let rev = ($(self.$index.resolve(pos + memoffset::offset_of_tuple!(Self::Archived, $index), &value.$index),)+);
                ($(rev.$index,)+)
            }
        }

        #[allow(non_snake_case)]
        impl<$($type: Archive),+> Archive for ($($type,)+) {
            type Archived = ($($type::Archived,)+);
            type Resolver = ($($type::Resolver,)+);

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
        unsafe impl<T: ArchiveSelf> ArchiveSelf for [T; $len] {}

        impl<T: Archive> Resolve<[T; $len]> for [T::Resolver; $len] {
            type Archived = [T::Archived; $len];

            fn resolve(self, pos: usize, value: &[T; $len]) -> Self::Archived {
                let mut resolvers = core::mem::MaybeUninit::new(self);
                let resolvers_ptr = resolvers.as_mut_ptr().cast::<T::Resolver>();
                let mut result = core::mem::MaybeUninit::<Self::Archived>::uninit();
                let result_ptr = result.as_mut_ptr().cast::<T::Archived>();
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
            type Archived = [T::Archived; $len];
            type Resolver = [T::Resolver; $len];

            fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
                let mut result = core::mem::MaybeUninit::<Self::Resolver>::uninit();
                let result_ptr = result.as_mut_ptr().cast::<T::Resolver>();
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
unsafe impl<T: ArchiveSelf, const N: usize> ArchiveSelf for [T; N] {}

#[cfg(feature = "const_generics")]
impl<T: Resolve<U>, U, const N: usize> Resolve<[U; N]> for [T; N] {
    type Archived = [T::Archived; N];

    fn resolve(self, pos: usize, value: &[U; N]) -> Self::Archived {
        let mut resolvers = core::mem::MaybeUninit::new(self);
        let resolvers_ptr = resolvers.as_mut_ptr().cast::<T>();
        let mut result = core::mem::MaybeUninit::<Self::Archived>::uninit();
        let result_ptr = result.as_mut_ptr().cast::<T::Archived>();
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
    type Archived = [T::Archived; N];
    type Resolver = [T::Resolver; N];

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        let mut result = core::mem::MaybeUninit::<[T::Resolver; N]>::uninit();
        let result_ptr = result.as_mut_ptr().cast::<T::Resolver>();
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

pub struct ArchivedStrRef {
    ptr: RelPtr<u8>,
    len: u32,
}

impl ArchivedStrRef {
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.as_ptr(), self.len as usize)
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            core::str::from_utf8_unchecked(self.as_bytes())
        }
    }
}

impl Resolve<str> for usize {
    type Archived = ArchivedStrRef;

    fn resolve(self, pos: usize, value: &str) -> Self::Archived {
        Self::Archived {
            ptr: RelPtr::new(pos + crate::offset_of!(Self::Archived, ptr), self),
            len: value.len() as u32,
        }
    }
}

impl ArchiveRef for str {
    type Archived = str;
    type Reference = ArchivedStrRef;
    type Resolver = usize;

    fn archive_ref<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        let result = writer.pos();
        writer.write(self.as_bytes())?;
        Ok(result)
    }
}

impl Deref for ArchivedStrRef {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Borrow<str> for ArchivedStrRef {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Hash for ArchivedStrRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl PartialEq for ArchivedStrRef {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl Eq for ArchivedStrRef {}

impl fmt::Display for ArchivedStrRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

pub struct ArchivedSliceRef<T> {
    ptr: RelPtr<T>,
    len: u32,
}

impl<T> ArchivedSliceRef<T> {
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe {
            core::slice::from_raw_parts(self.as_ptr(), self.len as usize)
        }
    }
}

impl<T: Archive> Resolve<[T]> for usize {
    type Archived = ArchivedSliceRef<T::Archived>;

    fn resolve(self, pos: usize, value: &[T]) -> Self::Archived {
        Self::Archived {
            ptr: RelPtr::new(pos + crate::offset_of!(Self::Archived, ptr), self),
            len: value.len() as u32,
        }
    }
}

#[cfg(any(not(feature = "std"), feature = "specialization"))]
impl<T: ArchiveSelf> ArchiveRef for [T] {
    #[cfg(not(feature = "std"))]
    type Archived = [T];
    #[cfg(not(feature = "std"))]
    type Reference = ArchivedSliceRef<T>;
    #[cfg(not(feature = "std"))]
    type Resolver = usize;

    fn archive_ref<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        use crate::WriteExt;

        let result = writer.align_for::<T>()?;
        let bytes = unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<u8>(), core::mem::size_of::<T>() * self.len()) };
        writer.write(bytes)?;
        Ok(result)
    }
}

impl<T> Deref for ArchivedSliceRef<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T: Hash> Hash for ArchivedSliceRef<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl<T: PartialEq> PartialEq for ArchivedSliceRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T: Eq> Eq for ArchivedSliceRef<T> {}

#[derive(Eq, PartialEq)]
#[repr(u8)]
pub enum ArchivedOption<T> {
    None,
    Some(T),
}

impl<T> ArchivedOption<T> {
    pub fn is_none(&self) -> bool {
        match self {
            ArchivedOption::None => true,
            ArchivedOption::Some(_) => false,
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            ArchivedOption::None => false,
            ArchivedOption::Some(_) => true,
        }
    }

    pub fn as_ref(&self) -> Option<&T> {
        match self {
            ArchivedOption::None => None,
            ArchivedOption::Some(value) => Some(value),
        }
    }

    pub fn unwrap(&self) -> &T {
        self.as_ref().unwrap()
    }
}

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedOptionTag {
    None,
    Some,
}

#[repr(C)]
struct ArchivedOptionVariantSome<T>(ArchivedOptionTag, T);

impl<T: Archive> Resolve<Option<T>> for Option<T::Resolver> {
    type Archived = ArchivedOption<T::Archived>;

    fn resolve(self, pos: usize, value: &Option<T>) -> Self::Archived {
        match self {
            None => ArchivedOption::None,
            Some(resolver) => ArchivedOption::Some(resolver.resolve(pos + crate::offset_of!(ArchivedOptionVariantSome<T::Archived>, 1), value.as_ref().unwrap())),
        }
    }
}

impl<T: Archive> Archive for Option<T> {
    type Archived = ArchivedOption<T::Archived>;
    type Resolver = Option<T::Resolver>;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        self.as_ref().map(|value| value.archive(writer)).transpose()
    }
}

impl<T: Hash> Hash for ArchivedOption<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<T, U: PartialEq<T>> PartialEq<Option<T>> for ArchivedOption<U> {
    fn eq(&self, other: &Option<T>) -> bool {
        if let ArchivedOption::Some(self_value) = self {
            if let Some(other_value) = other {
                self_value.eq(other_value)
            } else {
                false
            }
        } else {
            other.is_none()
        }
    }
}

impl<T: PartialEq<U>, U> PartialEq<ArchivedOption<T>> for Option<U> {
    fn eq(&self, other: &ArchivedOption<T>) -> bool {
        other.eq(self)
    }
}
