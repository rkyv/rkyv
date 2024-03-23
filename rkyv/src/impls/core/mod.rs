use core::{
    alloc::Layout,
    cell::{Cell, UnsafeCell},
    mem::ManuallyDrop,
    ptr, str,
};

use ptr_meta::Pointee;
use rancor::Fallible;

use crate::{
    primitive::ArchivedUsize,
    ser::{Allocator, Writer, WriterExt as _},
    tuple::*,
    Archive, ArchivePointee, ArchiveUnsized, ArchivedMetadata, Deserialize,
    DeserializeUnsized, Portable, Serialize, SerializeUnsized, CopyOptimization,
};

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
    ($name:ident, $($type:ident $index:tt),*) => {
        impl<$($type),*> Archive for ($($type,)*)
        where
            $($type: Archive,)*
        {
            type Archived = $name<$($type::Archived,)*>;
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

        impl<$($type,)* D> Deserialize<($($type,)*), D> for $name<$($type::Archived,)*>
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

impl_tuple!(ArchivedTuple1, T0 0);
impl_tuple!(ArchivedTuple2, T0 0, T1 1);
impl_tuple!(ArchivedTuple3, T0 0, T1 1, T2 2);
impl_tuple!(ArchivedTuple4, T0 0, T1 1, T2 2, T3 3);
impl_tuple!(ArchivedTuple5, T0 0, T1 1, T2 2, T3 3, T4 4);
impl_tuple!(ArchivedTuple6, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5);
impl_tuple!(ArchivedTuple7, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6);
impl_tuple!(ArchivedTuple8, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7);
impl_tuple!(ArchivedTuple9, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8);
impl_tuple!(ArchivedTuple10, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9);
impl_tuple!(ArchivedTuple11, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10);
impl_tuple!(
    ArchivedTuple12, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10, T11 11
);
impl_tuple!(
    ArchivedTuple13, T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10, T11 11,
    T12 12
);

impl<T: Archive, const N: usize> Archive for [T; N] {
    const COPY_OPTIMIZATION: CopyOptimization<Self> = unsafe {
        CopyOptimization::enable_if(T::COPY_OPTIMIZATION.is_enabled())
    };

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
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        if T::COPY_OPTIMIZATION.is_enabled() {
            let result = serializer.align_for::<T::Archived>()?;
            let as_bytes = unsafe {
                core::slice::from_raw_parts(
                    self.as_ptr().cast::<u8>(),
                    core::mem::size_of::<T>() * self.len(),
                )
            };
            serializer.write(as_bytes)?;

            Ok(result)
        } else {
            use crate::util::ScratchVec;

            let mut resolvers = unsafe {
                ScratchVec::new(serializer, self.len())?
            };

            for value in self.iter() {
                resolvers.push(value.serialize(serializer)?);
            }
            let result = serializer.align_for::<T::Archived>()?;
            for (value, resolver) in self.iter().zip(resolvers.drain(..)) {
                unsafe {
                    serializer.resolve_aligned(value, resolver)?;
                }
            }

            unsafe {
                resolvers.free(serializer)?;
            }

            Ok(result)
        }
    }
}

impl<T, U, D> DeserializeUnsized<[U], D> for [T]
where
    T: Deserialize<U, D>,
    D: Fallible + ?Sized,
{
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

    #[inline]
    fn deserialize_metadata(&self, _: &mut D) -> Result<<[U] as Pointee>::Metadata, D::Error> {
        Ok(ptr_meta::metadata(self))
    }
}

/// `str`

unsafe impl Portable for str {}

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

unsafe impl<T: Portable> Portable for ManuallyDrop<T> {}

impl<T: Archive> Archive for ManuallyDrop<T> {
    const COPY_OPTIMIZATION: CopyOptimization<Self> = unsafe {
        CopyOptimization::enable_if(T::COPY_OPTIMIZATION.is_enabled())
    };

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

// `Cell`

unsafe impl<T: Portable + ?Sized> Portable for Cell<T> {}

// `UnsafeCell`

unsafe impl<T: Portable + ?Sized> Portable for UnsafeCell<T> {}
