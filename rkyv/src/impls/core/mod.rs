#[cfg(feature = "copy")]
use crate::copy::ArchiveCopyOptimize;
use crate::{
    primitive::ArchivedUsize,
    ser::{Allocator, Writer, WriterExt as _},
    Archive, ArchivePointee, ArchiveUnsized, ArchivedMetadata, Deserialize,
    DeserializeUnsized, Serialize, SerializeUnsized,
};
use core::{alloc::Layout, mem::ManuallyDrop, ptr, str};
use ptr_meta::Pointee;
use rancor::Fallible;

mod ops;
mod option;
mod primitive;
mod result;
mod time;

impl<T> ArchivePointee for T {
    type ArchivedMetadata = ();

    #[inline]
    fn pointer_metadata(
        _: &Self::ArchivedMetadata,
    ) -> <Self as Pointee>::Metadata {
    }
}

impl<T: Archive> ArchiveUnsized for T {
    type Archived = T::Archived;

    #[inline]
    fn archived_metadata(&self) -> ArchivedMetadata<Self> {}
}

impl<T, S> SerializeUnsized<S> for T
where
    T: Serialize<S>,
    S: Fallible + Writer + ?Sized,
{
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        self.serialize_and_resolve(serializer)
    }
}

impl<T: Archive, D: Fallible + ?Sized> DeserializeUnsized<T, D> for T::Archived
where
    T::Archived: Deserialize<T, D>,
{
    #[inline]
    unsafe fn deserialize_unsized(
        &self,
        deserializer: &mut D,
        mut alloc: impl FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), D::Error> {
        let deserialized = self.deserialize(deserializer)?;

        let layout = Layout::new::<T>();
        if layout.size() == 0 {
            Ok(ptr::NonNull::<T>::dangling().as_ptr().cast())
        } else {
            let ptr = alloc(layout).cast::<T>();
            assert!(!ptr.is_null());
            ptr.write(deserialized);
            Ok(ptr.cast())
        }
    }

    #[inline]
    fn deserialize_metadata(
        &self,
        _: &mut D,
    ) -> Result<<T as Pointee>::Metadata, D::Error> {
        Ok(())
    }
}

macro_rules! impl_tuple {
    ($($type:ident $index:tt),*) => {
        #[cfg(not(feature = "strict"))]
        impl<$($type),*> Archive for ($($type,)*)
        where
            $($type: Archive,)*
        {
            type Archived = ($($type::Archived,)*);
            type Resolver = ($($type::Resolver,)*);

            #[inline]
            unsafe fn resolve(
                &self,
                pos: usize,
                resolver: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                $(
                    let (fp, fo) = out_field!(out.$index);
                    self.$index.resolve(pos + fp, resolver.$index, fo);
                )*
            }
        }

        #[cfg(not(feature = "strict"))]
        impl<$($type,)* S> Serialize<S> for ($($type,)*)
        where
            $($type: Serialize<S>,)*
            S: Fallible + ?Sized,
        {
            #[inline]
            fn serialize(
                &self,
                serializer: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                Ok((
                    $(self.$index.serialize(serializer)?,)*
                ))
            }
        }

        #[cfg(not(feature = "strict"))]
        impl<$($type,)* D> Deserialize<($($type,)*), D> for ($($type::Archived,)*)
        where
            D: Fallible + ?Sized,
            $($type: Archive,)*
            $($type::Archived: Deserialize<$type, D>,)*
        {
            #[inline]
            fn deserialize(
                &self,
                deserializer: &mut D,
            ) -> Result<($($type,)*), D::Error> {
                Ok((
                    $(self.$index.deserialize(deserializer)?,)*
                ))
            }
        }
    };
}

impl_tuple!(T0 0);
impl_tuple!(T0 0, T1 1);
impl_tuple!(T0 0, T1 1, T2 2);
impl_tuple!(T0 0, T1 1, T2 2, T3 3);
impl_tuple!(T0 0, T1 1, T2 2, T3 3, T4 4);
impl_tuple!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5);
impl_tuple!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6);
impl_tuple!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7);
impl_tuple!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8);
impl_tuple!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9);
impl_tuple!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10);
impl_tuple!(
    T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10, T11 11
);
impl_tuple!(
    T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10, T11 11,
    T12 12
);

impl<T: Archive, const N: usize> Archive for [T; N] {
    type Archived = [T::Archived; N];
    type Resolver = [T::Resolver; N];

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let mut resolvers = core::mem::MaybeUninit::new(resolver);
        let resolvers_ptr = resolvers.as_mut_ptr().cast::<T::Resolver>();
        let out_ptr = out.cast::<T::Archived>();
        for (i, value) in self.iter().enumerate() {
            value.resolve(
                pos + i * core::mem::size_of::<T::Archived>(),
                resolvers_ptr.add(i).read(),
                out_ptr.add(i),
            );
        }
    }
}

impl<T, S, const N: usize> Serialize<S> for [T; N]
where
    T: Serialize<S>,
    S: Fallible + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
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

impl<T, D, const N: usize> Deserialize<[T; N], D> for [T::Archived; N]
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
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

    fn archived_metadata(&self) -> ArchivedMetadata<Self> {
        ArchivedUsize::from_native(ptr_meta::metadata(self) as _)
    }
}

impl<T> ArchivePointee for [T] {
    type ArchivedMetadata = ArchivedUsize;

    #[inline]
    fn pointer_metadata(
        archived: &Self::ArchivedMetadata,
    ) -> <Self as Pointee>::Metadata {
        archived.to_native() as usize
    }
}

impl<T, S> SerializeUnsized<S> for [T]
where
    T: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    default! {
        fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
            use crate::util::ScratchVec;

            unsafe {
                let mut resolvers = ScratchVec::new(serializer, self.len())?;

                for value in self.iter() {
                    resolvers.push(value.serialize(serializer)?);
                }
                let result = serializer.align_for::<T::Archived>()?;
                for (value, resolver) in self.iter().zip(resolvers.drain(..)) {
                    serializer.resolve_aligned(value, resolver)?;
                }

                resolvers.free(serializer)?;

                Ok(result)
            }
        }
    }
}

#[cfg(feature = "copy")]
impl<T, S> SerializeUnsized<S> for [T]
where
    T: Serialize<S> + crate::copy::ArchiveCopyOptimize,
    S: Allocator + Writer + ?Sized,
{
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, E> {
        unsafe {
            let result = serializer.align_for::<T>()?;
            if !self.is_empty() {
                let bytes = core::slice::from_raw_parts(
                    (self.as_ptr() as *const T).cast::<u8>(),
                    self.len() * core::mem::size_of::<T>(),
                );
                serializer.write(bytes)?;
            }
            Ok(result)
        }
    }

    #[inline]
    fn serialize_metadata(
        &self,
        _: &mut S,
    ) -> Result<Self::MetadataResolver, E> {
        Ok(())
    }
}

impl<T, U, D> DeserializeUnsized<[U], D> for [T]
where
    T: Deserialize<U, D>,
    D: Fallible + ?Sized,
{
    default! {
        unsafe fn deserialize_unsized(&self, deserializer: &mut D, mut alloc: impl FnMut(Layout) -> *mut u8) -> Result<*mut (), D::Error> {
            if self.is_empty() || core::mem::size_of::<U>() == 0 {
                Ok(ptr::NonNull::<U>::dangling().as_ptr().cast())
            } else {
                let result = alloc(Layout::array::<U>(self.len()).unwrap()).cast::<U>();
                assert!(!result.is_null());
                for (i, item) in self.iter().enumerate() {
                    result.add(i).write(item.deserialize(deserializer)?);
                }
                Ok(result.cast())
            }
        }
    }

    default! {
        #[inline]
        fn deserialize_metadata(&self, _: &mut D) -> Result<<[U] as Pointee>::Metadata, D::Error> {
            Ok(ptr_meta::metadata(self))
        }
    }
}

#[cfg(feature = "copy")]
impl<T, U, D> DeserializeUnsized<[U], D> for [T]
where
    T: Deserialize<U, D>,
    U: ArchiveCopyOptimize,
    D: Fallible + ?Sized,
{
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        mut alloc: impl FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), E> {
        if self.is_empty() || core::mem::size_of::<T>() == 0 {
            Ok(ptr::NonNull::<U>::dangling().as_ptr().cast())
        } else {
            let result =
                alloc(Layout::array::<T>(self.len()).unwrap()).cast::<T>();
            assert!(!result.is_null());
            ptr::copy_nonoverlapping(self.as_ptr(), result, self.len());
            Ok(result.cast())
        }
    }

    #[inline]
    fn deserialize_metadata(
        &self,
        _: &mut D,
    ) -> Result<<[T] as Pointee>::Metadata, E> {
        Ok(ptr_meta::metadata(self))
    }
}

/// `str`

impl ArchiveUnsized for str {
    type Archived = str;

    #[inline]
    fn archived_metadata(&self) -> ArchivedMetadata<Self> {
        ArchivedUsize::from_native(ptr_meta::metadata(self) as _)
    }
}

impl ArchivePointee for str {
    type ArchivedMetadata = ArchivedUsize;

    #[inline]
    fn pointer_metadata(
        archived: &Self::ArchivedMetadata,
    ) -> <Self as Pointee>::Metadata {
        <[u8]>::pointer_metadata(archived)
    }
}

impl<S: Fallible + Writer + ?Sized> SerializeUnsized<S> for str {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        let result = serializer.pos();
        serializer.write(self.as_bytes())?;
        Ok(result)
    }
}

impl<D: Fallible + ?Sized> DeserializeUnsized<str, D> for str {
    #[inline]
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        mut alloc: impl FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), D::Error> {
        if self.is_empty() {
            Ok(ptr::NonNull::dangling().as_ptr())
        } else {
            let bytes = alloc(Layout::array::<u8>(self.len()).unwrap());
            assert!(!bytes.is_null());
            ptr::copy_nonoverlapping(self.as_ptr(), bytes, self.len());
            Ok(bytes.cast())
        }
    }

    #[inline]
    fn deserialize_metadata(
        &self,
        _: &mut D,
    ) -> Result<<str as Pointee>::Metadata, D::Error> {
        Ok(ptr_meta::metadata(self))
    }
}

// `ManuallyDrop`

impl<T: Archive> Archive for ManuallyDrop<T> {
    type Archived = ManuallyDrop<T::Archived>;
    type Resolver = T::Resolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        T::resolve(self, pos, resolver, out.cast::<T::Archived>())
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for ManuallyDrop<T> {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        T::serialize(self, serializer)
    }
}

impl<T, D> Deserialize<ManuallyDrop<T>, D> for ManuallyDrop<T::Archived>
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<ManuallyDrop<T>, D::Error> {
        T::Archived::deserialize(self, deserializer).map(ManuallyDrop::new)
    }
}
