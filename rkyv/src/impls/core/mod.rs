#[cfg(feature = "copy")]
use crate::copy::ArchiveCopyOptimize;
use crate::{
    primitive::ArchivedUsize,
    ser::{ScratchSpace, Serializer},
    Archive, ArchivePointee, ArchiveUnsized, ArchivedMetadata, Deserialize,
    DeserializeUnsized, Serialize, SerializeUnsized,
};
use core::{alloc::Layout, mem::ManuallyDrop, ptr, str};
use ptr_meta::Pointee;

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

    type MetadataResolver = ();

    #[inline]
    unsafe fn resolve_metadata(
        &self,
        _: usize,
        _: Self::MetadataResolver,
        _: *mut ArchivedMetadata<Self>,
    ) {
    }
}

impl<T: Serialize<S, E>, S: Serializer<E> + ?Sized, E> SerializeUnsized<S, E> for T {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, E> {
        serializer.serialize_value(self)
    }

    #[inline]
    fn serialize_metadata(&self, _: &mut S) -> Result<(), E> {
        Ok(())
    }
}

impl<T: Archive, D: ?Sized, E> DeserializeUnsized<T, D, E> for T::Archived
where
    T::Archived: Deserialize<T, D, E>,
{
    #[inline]
    unsafe fn deserialize_unsized(
        &self,
        deserializer: &mut D,
        mut alloc: impl FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), E> {
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
    ) -> Result<<T as Pointee>::Metadata, E> {
        Ok(())
    }
}

// TODO: Correct the mistakes of your past. Serialize tuple elements lowest to highest.

#[cfg(not(feature = "strict"))]
macro_rules! peel_tuple {
    ($type:ident $index:tt, $($type_rest:ident $index_rest:tt,)*) => { impl_tuple! { $($type_rest $index_rest,)* } };
}

#[cfg(not(feature = "strict"))]
macro_rules! impl_tuple {
    () => ();
    ($($type:ident $index:tt,)+) => {
        impl<$($type: Archive),+> Archive for ($($type,)+) {
            type Archived = ($($type::Archived,)+);
            type Resolver = ($($type::Resolver,)+);

            #[inline]
            unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
                $(
                    let (fp, fo) = out_field!(out.$index);
                    self.$index.resolve(pos + fp, resolver.$index, fo);
                )+
            }
        }

        impl<$($type: Serialize<S>),+, S: ?Sized> Serialize<S> for ($($type,)+) {
            #[inline]
            fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, E> {
                let rev = ($(self.$index.serialize(serializer)?,)+);
                Ok(($(rev.$index,)+))
            }
        }

        impl<D: ?Sized, $($type: Archive),+> Deserialize<($($type,)+), D> for ($($type::Archived,)+)
        where
            $($type::Archived: Deserialize<$type, D>,)+
        {
            #[inline]
            fn deserialize(&self, deserializer: &mut D) -> Result<($($type,)+), E> {
                let rev = ($(&self.$index,)+);
                Ok(($(rev.$index.deserialize(deserializer)?,)+))
            }
        }

        peel_tuple! { $($type $index,)+ }
    };
}

#[cfg(not(feature = "strict"))]
impl_tuple! { T11 11, T10 10, T9 9, T8 8, T7 7, T6 6, T5 5, T4 4, T3 3, T2 2, T1 1, T0 0, }

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

impl<T: Serialize<S, E>, S: ?Sized, const N: usize, E> Serialize<S, E>
    for [T; N]
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
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

impl<T: Archive, D: ?Sized, const N: usize, E> Deserialize<[T; N], D, E>
    for [T::Archived; N]
where
    T::Archived: Deserialize<T, D, E>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<[T; N], E> {
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
    unsafe fn resolve_metadata(
        &self,
        _: usize,
        _: Self::MetadataResolver,
        out: *mut ArchivedMetadata<Self>,
    ) {
        out.write(ArchivedUsize::from_native(ptr_meta::metadata(self) as _));
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

impl<T: Serialize<S, E>, S: ScratchSpace<E> + Serializer<E> + ?Sized, E> SerializeUnsized<S, E>
    for [T]
{
    default! {
        fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, E> {
            use crate::ScratchVec;

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

    default! {
        #[inline]
        fn serialize_metadata(&self, _: &mut S) -> Result<Self::MetadataResolver, E> {
            Ok(())
        }
    }
}

#[cfg(feature = "copy")]
impl<T, S> SerializeUnsized<S> for [T]
where
    T: Serialize<S> + crate::copy::ArchiveCopyOptimize,
    S: ScratchSpace + Serializer + ?Sized,
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

impl<T: Deserialize<U, D, E>, U, D: ?Sized, E> DeserializeUnsized<[U], D, E>
    for [T]
{
    default! {
        unsafe fn deserialize_unsized(&self, deserializer: &mut D, mut alloc: impl FnMut(Layout) -> *mut u8) -> Result<*mut (), E> {
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
        fn deserialize_metadata(&self, _: &mut D) -> Result<<[U] as Pointee>::Metadata, E> {
            Ok(ptr_meta::metadata(self))
        }
    }
}

#[cfg(feature = "copy")]
impl<T, U, D> DeserializeUnsized<[U], D> for [T]
where
    T: Deserialize<U, D>,
    U: ArchiveCopyOptimize,
    D: ?Sized,
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

    type MetadataResolver = ();

    #[inline]
    unsafe fn resolve_metadata(
        &self,
        _: usize,
        _: Self::MetadataResolver,
        out: *mut ArchivedMetadata<Self>,
    ) {
        out.write(ArchivedUsize::from_native(ptr_meta::metadata(self) as _))
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

impl<S: Serializer<E> + ?Sized, E> SerializeUnsized<S, E> for str {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, E> {
        let result = serializer.pos();
        serializer.write(self.as_bytes())?;
        Ok(result)
    }

    #[inline]
    fn serialize_metadata(
        &self,
        _: &mut S,
    ) -> Result<Self::MetadataResolver, E> {
        Ok(())
    }
}

impl<D: ?Sized, E> DeserializeUnsized<str, D, E> for str {
    #[inline]
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        mut alloc: impl FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), E> {
        if self.is_empty() {
            Ok(ptr::null_mut())
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
    ) -> Result<<str as Pointee>::Metadata, E> {
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

impl<T: Serialize<S, E>, S: ?Sized, E> Serialize<S, E> for ManuallyDrop<T> {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        T::serialize(self, serializer)
    }
}

impl<T: Archive, D: ?Sized, E> Deserialize<ManuallyDrop<T>, D, E>
    for ManuallyDrop<T::Archived>
where
    T::Archived: Deserialize<T, D, E>,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<ManuallyDrop<T>, E> {
        T::Archived::deserialize(self, deserializer).map(ManuallyDrop::new)
    }
}
