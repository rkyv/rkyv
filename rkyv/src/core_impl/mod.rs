//! [`Archive`] implementations for core types.

use crate::{
    de::Deserializer, offset_of, project_struct, ser::Serializer, Archive, ArchiveCopy,
    ArchivePointee, ArchiveUnsized, Archived, ArchivedMetadata, ArchivedUsize,
    Deserialize, DeserializeUnsized, Fallible, FixedUsize, Serialize, SerializeUnsized,
};
use core::{
    alloc, cmp,
    hash::{Hash, Hasher},
    mem::MaybeUninit,
    ptr, str,
};
use ptr_meta::Pointee;

pub mod primitive;
pub mod range;
pub mod time;

impl<T> ArchivePointee for T {
    type ArchivedMetadata = ();

    #[inline]
    fn pointer_metadata(_: &Self::ArchivedMetadata) -> <Self as Pointee>::Metadata {}
}

impl<T: Archive> ArchiveUnsized for T {
    type Archived = T::Archived;

    type MetadataResolver = ();

    #[inline]
    fn resolve_metadata(
        &self,
        _: usize,
        _: Self::MetadataResolver,
        _: &mut MaybeUninit<ArchivedMetadata<Self>>,
    ) {
    }
}

impl<T: Serialize<S>, S: Serializer + ?Sized> SerializeUnsized<S> for T {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        serializer.serialize_value(self)
    }

    #[inline]
    fn serialize_metadata(&self, _: &mut S) -> Result<ArchivedMetadata<Self>, S::Error> {
        Ok(())
    }
}

impl<T: Archive, D: Deserializer + ?Sized> DeserializeUnsized<T, D> for T::Archived
where
    T::Archived: Deserialize<T, D>,
{
    #[inline]
    unsafe fn deserialize_unsized(&self, deserializer: &mut D) -> Result<*mut (), D::Error> {
        let ptr = deserializer.alloc(alloc::Layout::new::<T>())?.cast::<T>();
        let deserialized = self.deserialize(deserializer)?;
        ptr.write(deserialized);
        Ok(ptr.cast())
    }

    #[inline]
    fn deserialize_metadata(&self, _: &mut D) -> Result<<T as Pointee>::Metadata, D::Error> {
        Ok(())
    }
}

#[cfg(not(feature = "strict"))]
macro_rules! peel_tuple {
    ($type:ident $index:tt, $($type_rest:ident $index_rest:tt,)*) => { impl_tuple! { $($type_rest $index_rest,)* } };
}

#[cfg(not(feature = "strict"))]
macro_rules! impl_tuple {
    () => ();
    ($($type:ident $index:tt,)+) => {
        unsafe impl<$($type: ArchiveCopy),+> ArchiveCopy for ($($type,)+) {}

        impl<$($type: Archive),+> Archive for ($($type,)+) {
            type Archived = ($($type::Archived,)+);
            type Resolver = ($($type::Resolver,)+);

            #[inline]
            fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                $(
                    #[allow(clippy::unneeded_wildcard_pattern)]
                    self.$index.resolve(
                        pos + memoffset::offset_of_tuple!(Self::Archived, $index),
                        resolver.$index,
                        crate::project_tuple!(out: Self::Archived => $index)
                    );
                )+
            }
        }

        impl<$($type: Serialize<S>),+, S: Fallible + ?Sized> Serialize<S> for ($($type,)+) {
            #[inline]
            fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
                let rev = ($(self.$index.serialize(serializer)?,)+);
                Ok(($(rev.$index,)+))
            }
        }

        impl<D: Fallible + ?Sized, $($type: Archive),+> Deserialize<($($type,)+), D> for ($($type::Archived,)+)
        where
            $($type::Archived: Deserialize<$type, D>,)+
        {
            #[inline]
            fn deserialize(&self, deserializer: &mut D) -> Result<($($type,)+), D::Error> {
                let rev = ($(&self.$index,)+);
                Ok(($(rev.$index.deserialize(deserializer)?,)+))
            }
        }

        peel_tuple! { $($type $index,)+ }
    };
}

#[cfg(not(feature = "strict"))]
impl_tuple! { T11 11, T10 10, T9 9, T8 8, T7 7, T6 6, T5 5, T4 4, T3 3, T2 2, T1 1, T0 0, }

#[cfg(not(feature = "const_generics"))]
macro_rules! impl_array {
    () => ();
    ($len:literal, $($rest:literal,)*) => {
        unsafe impl<T: ArchiveCopy> ArchiveCopy for [T; $len] {}

        impl<T: Archive> Archive for [T; $len] {
            type Archived = [T::Archived; $len];
            type Resolver = [T::Resolver; $len];

            #[inline]
            fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
                let mut resolvers = core::mem::MaybeUninit::new(resolver);
                let resolvers_ptr = resolvers.as_mut_ptr().cast::<T::Resolver>();
                let out_ptr = out.as_mut_ptr().cast::<MaybeUninit<T::Archived>>();
                #[allow(clippy::reversed_empty_ranges)]
                for (i, value) in self.iter().enumerate() {
                    unsafe {
                        value.resolve(
                            pos + i * core::mem::size_of::<T::Archived>(),
                            resolvers_ptr.add(i).read(),
                            &mut *out_ptr.add(i),
                        );
                    }
                }
            }
        }

        impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for [T; $len] {
            #[inline]
            fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
                let mut result = core::mem::MaybeUninit::<Self::Resolver>::uninit();
                let result_ptr = result.as_mut_ptr().cast::<T::Resolver>();
                #[allow(clippy::reversed_empty_ranges)]
                for i in 0..$len {
                    unsafe {
                        result_ptr.add(i).write(self[i].serialize(serializer)?);
                    }
                }
                unsafe { Ok(result.assume_init()) }
            }
        }

        impl<T: Archive, D: Fallible + ?Sized> Deserialize<[T; $len], D> for [T::Archived; $len]
        where
            T::Archived: Deserialize<T, D>,
        {
            #[inline]
            fn deserialize(&self, deserializer: &mut D) -> Result<[T; $len], D::Error> {
                let mut result = core::mem::MaybeUninit::<[T; $len]>::uninit();
                let result_ptr = result.as_mut_ptr().cast::<T>();
                #[allow(clippy::reversed_empty_ranges)]
                for i in 0..$len {
                    unsafe {
                        result_ptr.add(i).write(self[i].deserialize(deserializer)?);
                    }
                }
                unsafe { Ok(result.assume_init()) }
            }
        }

        impl_array! { $($rest,)* }
    };
}

#[cfg(not(feature = "const_generics"))]
impl_array! { 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, }

#[cfg(feature = "const_generics")]
unsafe impl<T: ArchiveCopy, const N: usize> ArchiveCopy for [T; N] {}

#[cfg(feature = "const_generics")]
impl<T: Archive, const N: usize> Archive for [T; N] {
    type Archived = [T::Archived; N];
    type Resolver = [T::Resolver; N];

    #[inline]
    fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        let mut resolvers = core::mem::MaybeUninit::new(resolver);
        let resolvers_ptr = resolvers.as_mut_ptr().cast::<T::Resolver>();
        let out_ptr = out.as_mut_ptr().cast::<MaybeUninit<T::Archived>>();
        for (i, value) in self.iter().enumerate() {
            unsafe {
                value.resolve(
                    pos + i * core::mem::size_of::<T::Archived>(),
                    resolvers_ptr.add(i).read(),
                    &mut *out_ptr.add(i),
                );
            }
        }
    }
}

#[cfg(feature = "const_generics")]
impl<T: Serialize<S>, S: Fallible + ?Sized, const N: usize> Serialize<S> for [T; N] {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let mut result = core::mem::MaybeUninit::<Self::Resolver>::uninit();
        let result_ptr = result.as_mut_ptr().cast::<T::Resolver>();
        for (i, value) in self.iter().enumerate() {
            unsafe {
                result_ptr.add(i).write(value.serialize(serializer)?);
            }
        }
        unsafe { Ok(result.assume_init()) }
    }
}

#[cfg(feature = "const_generics")]
impl<T: Archive, D: Fallible + ?Sized, const N: usize> Deserialize<[T; N], D> for [T::Archived; N]
where
    T::Archived: Deserialize<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<[T; N], D::Error> {
        let mut result = core::mem::MaybeUninit::<[T; N]>::uninit();
        let result_ptr = result.as_mut_ptr().cast::<T>();
        for (i, value) in self.iter().enumerate() {
            unsafe {
                result_ptr.add(i).write(value.deserialize(deserializer)?);
            }
        }
        unsafe { Ok(result.assume_init()) }
    }
}

impl<T: Archive> ArchiveUnsized for [T] {
    type Archived = [T::Archived];

    type MetadataResolver = ();

    #[inline]
    fn resolve_metadata(
        &self,
        _: usize,
        _: Self::MetadataResolver,
        out: &mut MaybeUninit<ArchivedMetadata<Self>>,
    ) {
        unsafe {
            out.as_mut_ptr().write(ArchivedUsize::from(ptr_meta::metadata(self) as FixedUsize));
        }
    }
}

impl<T> ArchivePointee for [T] {
    type ArchivedMetadata = Archived<usize>;

    #[inline]
    fn pointer_metadata(archived: &Self::ArchivedMetadata) -> <Self as Pointee>::Metadata {
        FixedUsize::from(*archived) as usize
    }
}

#[cfg(any(not(feature = "std"), feature = "specialization"))]
impl<T: ArchiveCopy + Serialize<S>, S: Serializer + ?Sized> SerializeUnsized<S> for [T] {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        if self.is_empty() || core::mem::size_of::<T::Archived>() == 0 {
            Ok(0)
        } else {
            unsafe {
                let bytes = core::slice::from_raw_parts(
                    (self.as_ptr() as *const T).cast::<u8>(),
                    self.len() * core::mem::size_of::<T>(),
                );
                let result = serializer.align_for::<T>()?;
                serializer.write(bytes)?;
                Ok(result)
            }
        }
    }

    #[inline]
    fn serialize_metadata(&self, _: &mut S) -> Result<Self::MetadataResolver, S::Error> {
        Ok(())
    }
}

#[cfg(any(feature = "std", feature = "specialization"))]
impl<T: Serialize<S>, S: Serializer + ?Sized> SerializeUnsized<S> for [T] {
    #[inline]
    default! {
        fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
            if self.is_empty() || core::mem::size_of::<T::Archived>() == 0 {
                Ok(0)
            } else {
                let mut resolvers = Vec::with_capacity(self.len());
                for value in self {
                    resolvers.push(value.serialize(serializer)?);
                }
                let result = serializer.align_for::<T::Archived>()?;
                unsafe {
                    for (i, resolver) in resolvers.drain(..).enumerate() {
                        serializer.resolve_aligned(&self[i], resolver)?;
                    }
                }
                Ok(result)
            }
        }
    }

    #[inline]
    default! {
        fn serialize_metadata(&self, _: &mut S) -> Result<Self::MetadataResolver, S::Error> {
            Ok(())
        }
    }
}

#[cfg(any(not(feature = "std"), feature = "specialization"))]
impl<T: Deserialize<T, D> + ArchiveCopy, D: Deserializer + ?Sized> DeserializeUnsized<[T], D>
    for [T]
{
    #[inline]
    unsafe fn deserialize_unsized(&self, deserializer: &mut D) -> Result<*mut (), D::Error> {
        if self.is_empty() || core::mem::size_of::<T>() == 0 {
            Ok(ptr::NonNull::dangling().as_ptr())
        } else {
            let result = deserializer
                .alloc(alloc::Layout::array::<T>(self.len()).unwrap())?
                .cast::<T>();
            ptr::copy_nonoverlapping(self.as_ptr(), result, self.len());
            Ok(result.cast())
        }
    }

    #[inline]
    fn deserialize_metadata(&self, _: &mut D) -> Result<<[T] as Pointee>::Metadata, D::Error> {
        Ok(ptr_meta::metadata(self))
    }
}

#[cfg(any(feature = "std", feature = "specialization"))]
impl<T: Deserialize<U, D>, U: Archive<Archived = T>, D: Deserializer + ?Sized>
    DeserializeUnsized<[U], D> for [T]
{
    #[inline]
    default! {
        unsafe fn deserialize_unsized(&self, deserializer: &mut D) -> Result<*mut (), D::Error> {
            if self.is_empty() || core::mem::size_of::<U>() == 0 {
                Ok(ptr::NonNull::dangling().as_ptr())
            } else {
                let result = deserializer
                    .alloc(alloc::Layout::array::<U>(self.len()).unwrap())?
                    .cast::<U>();
                for (i, item) in self.iter().enumerate() {
                    result.add(i).write(item.deserialize(deserializer)?);
                }
                Ok(result.cast())
            }
        }
    }

    #[inline]
    default! {
        fn deserialize_metadata(&self, _: &mut D) -> Result<<[U] as Pointee>::Metadata, D::Error> {
            Ok(ptr_meta::metadata(self))
        }
    }
}

impl ArchiveUnsized for str {
    type Archived = str;

    type MetadataResolver = ();

    #[inline]
    fn resolve_metadata(
        &self,
        _: usize,
        _: Self::MetadataResolver,
        out: &mut MaybeUninit<ArchivedMetadata<Self>>,
    ) {
        unsafe {
            out.as_mut_ptr().write(ArchivedUsize::from(ptr_meta::metadata(self) as FixedUsize));
        }
    }
}

impl ArchivePointee for str {
    type ArchivedMetadata = Archived<usize>;

    #[inline]
    fn pointer_metadata(archived: &Self::ArchivedMetadata) -> <Self as Pointee>::Metadata {
        <[u8]>::pointer_metadata(archived)
    }
}

impl<S: Serializer + ?Sized> SerializeUnsized<S> for str {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        let result = serializer.pos();
        serializer.write(self.as_bytes())?;
        Ok(result)
    }

    #[inline]
    fn serialize_metadata(&self, _: &mut S) -> Result<Self::MetadataResolver, S::Error> {
        Ok(())
    }
}

impl<D: Deserializer + ?Sized> DeserializeUnsized<str, D> for <str as ArchiveUnsized>::Archived {
    #[inline]
    unsafe fn deserialize_unsized(&self, deserializer: &mut D) -> Result<*mut (), D::Error> {
        let bytes = deserializer.alloc(alloc::Layout::array::<u8>(self.len()).unwrap())?;
        ptr::copy_nonoverlapping(self.as_ptr(), bytes, self.len());
        Ok(bytes.cast())
    }

    #[inline]
    fn deserialize_metadata(&self, _: &mut D) -> Result<<str as Pointee>::Metadata, D::Error> {
        Ok(ptr_meta::metadata(self))
    }
}

/// An archived [`Option`].
///
/// It functions identically to [`Option`] but has a different internal
/// representation to allow for archiving.
#[derive(Debug)]
#[cfg_attr(feature = "validation", derive(bytecheck::CheckBytes))]
#[repr(u8)]
pub enum ArchivedOption<T> {
    /// No value
    None,
    /// Some value `T`
    Some(T),
}

impl<T> ArchivedOption<T> {
    /// Returns `true` if the option is a `None` value.
    #[inline]
    pub fn is_none(&self) -> bool {
        match self {
            ArchivedOption::None => true,
            ArchivedOption::Some(_) => false,
        }
    }

    /// Returns `true` if the option is a `Some` value.
    #[inline]
    pub fn is_some(&self) -> bool {
        match self {
            ArchivedOption::None => false,
            ArchivedOption::Some(_) => true,
        }
    }

    /// Converts to an `Option<&T>`.
    #[inline]
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            ArchivedOption::None => None,
            ArchivedOption::Some(value) => Some(value),
        }
    }

    /// Converts to an `Option<&mut T>`.
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            ArchivedOption::None => None,
            ArchivedOption::Some(value) => Some(value),
        }
    }

    /// Inserts `v` into the option if it is `None`, then returns a mutable
    /// reference to the contained value.
    #[inline]
    pub fn get_or_insert(&mut self, v: T) -> &mut T {
        self.get_or_insert_with(move || v)
    }

    /// Inserts a value computed from `f` into the option if it is `None`, then
    /// returns a mutable reference to the contained value.
    #[inline]
    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if let ArchivedOption::Some(ref mut value) = self {
            value
        } else {
            *self = ArchivedOption::Some(f());
            self.as_mut().unwrap()
        }
    }
}

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedOptionTag {
    None,
    Some,
}

#[repr(C)]
struct ArchivedOptionVariantNone(ArchivedOptionTag);

#[repr(C)]
struct ArchivedOptionVariantSome<T>(ArchivedOptionTag, T);

impl<T: Archive> Archive for Option<T> {
    type Archived = ArchivedOption<T::Archived>;
    type Resolver = Option<T::Resolver>;

    #[inline]
    fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        unsafe {
            match resolver {
                None => {
                    let out = &mut *out
                        .as_mut_ptr()
                        .cast::<MaybeUninit<ArchivedOptionVariantNone>>();
                    project_struct!(out: ArchivedOptionVariantNone => 0: ArchivedOptionTag)
                        .as_mut_ptr()
                        .write(ArchivedOptionTag::None);
                }
                Some(resolver) => {
                    let out = &mut *out
                        .as_mut_ptr()
                        .cast::<MaybeUninit<ArchivedOptionVariantSome<T::Archived>>>();
                    project_struct!(out: ArchivedOptionVariantSome<T::Archived> => 0: ArchivedOptionTag)
                        .as_mut_ptr()
                        .write(ArchivedOptionTag::Some);
                    self.as_ref().unwrap().resolve(
                        pos + offset_of!(ArchivedOptionVariantSome<T::Archived>, 1),
                        resolver,
                        project_struct!(out: ArchivedOptionVariantSome<T::Archived> => 1),
                    );
                }
            }
        }
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Option<T> {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        self.as_ref()
            .map(|value| value.serialize(serializer))
            .transpose()
    }
}

impl<T: Archive, D: Fallible + ?Sized> Deserialize<Option<T>, D> for Archived<Option<T>>
where
    T::Archived: Deserialize<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Option<T>, D::Error> {
        match self {
            ArchivedOption::Some(value) => Ok(Some(value.deserialize(deserializer)?)),
            ArchivedOption::None => Ok(None),
        }
    }
}

impl<T: Eq> Eq for ArchivedOption<T> {}

impl<T: Hash> Hash for ArchivedOption<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<T: Ord> Ord for ArchivedOption<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(&other.as_ref())
    }
}

impl<T: PartialEq> PartialEq for ArchivedOption<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<T: PartialOrd> PartialOrd for ArchivedOption<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ref().partial_cmp(&other.as_ref())
    }
}

impl<T, U: PartialEq<T>> PartialEq<Option<T>> for ArchivedOption<U> {
    #[inline]
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
    #[inline]
    fn eq(&self, other: &ArchivedOption<T>) -> bool {
        other.eq(self)
    }
}
