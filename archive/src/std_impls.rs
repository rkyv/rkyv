use core::ops::Deref;
use crate::{
    Archive,
    ArchiveRef,
    rel_ptr,
    RelPtr,
    Resolve,
    Write,
};

pub struct ArchivedString(<str as ArchiveRef>::Reference);

impl Deref for ArchivedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PartialEq<String> for ArchivedString {
    fn eq(&self, other: &String) -> bool {
        self.deref().eq(other.deref())
    }
}

impl PartialEq<ArchivedString> for String {
    fn eq(&self, other: &ArchivedString) -> bool {
        other.eq(self)
    }
}

pub struct StringResolver(<str as ArchiveRef>::Resolver);

impl Resolve<String> for StringResolver {
    type Archived = ArchivedString;

    fn resolve(self, pos: usize, value: &String) -> Self::Archived {
        ArchivedString(self.0.resolve(pos, value.as_str()))
    }
}

impl Archive for String {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(StringResolver(self.as_str().archive_ref(writer)?))
    }
}

pub struct ArchivedBox<T>(T);

impl<T: Deref> Deref for ArchivedBox<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: Deref<Target = U>, U: PartialEq<V> + ?Sized, V: ?Sized> PartialEq<Box<V>> for ArchivedBox<T> {
    fn eq(&self, other: &Box<V>) -> bool {
        self.deref().eq(other.deref())
    }
}

pub struct BoxResolver<T>(T);

impl<T: ArchiveRef + ?Sized> Resolve<Box<T>> for BoxResolver<T::Resolver> {
    type Archived = ArchivedBox<T::Reference>;

    fn resolve(self, pos: usize, value: &Box<T>) -> Self::Archived {
        ArchivedBox(self.0.resolve(pos, value.as_ref()))
    }
}

impl<T: ArchiveRef + ?Sized> Archive for Box<T> {
    type Archived = ArchivedBox<T::Reference>;
    type Resolver = BoxResolver<T::Resolver>;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(BoxResolver(self.as_ref().archive_ref(writer)?))
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

impl<T> Deref for ArchivedSliceRef<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T: Archive> Resolve<[T]> for usize {
    type Archived = ArchivedSliceRef<T::Archived>;

    fn resolve(self, pos: usize, value: &[T]) -> Self::Archived {
        Self::Archived {
            ptr: rel_ptr!(pos, self, Self::Archived, ptr),
            len: value.len() as u32,
        }
    }
}

impl<T: Archive> ArchiveRef for [T] {
    type Archived = [T::Archived];
    type Reference = ArchivedSliceRef<T::Archived>;
    type Resolver = usize;

    fn archive_ref<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        let mut resolvers = Vec::with_capacity(self.len());
        for i in 0..self.len() {
            resolvers.push(self[i].archive(writer)?);
        }
        let result = writer.align_for::<T::Archived>()?;
        unsafe {
            for (i, resolver) in resolvers.drain(..).enumerate() {
                writer.resolve_aligned(&self[i], resolver)?;
            }
        }
        Ok(result)
    }
}

pub struct ArchivedVec<T>(T);

impl<T: Deref> Deref for ArchivedVec<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub struct VecResolver<T>(T);

impl<T: Resolve<[U]>, U> Resolve<Vec<U>> for VecResolver<T> {
    type Archived = ArchivedVec<T::Archived>;

    fn resolve(self, pos: usize, value: &Vec<U>) -> Self::Archived {
        ArchivedVec(self.0.resolve(pos, value.deref()))
    }
}

impl<T: Archive> Archive for Vec<T> {
    type Archived = ArchivedVec<<[T] as ArchiveRef>::Reference>;
    type Resolver = VecResolver<<[T] as ArchiveRef>::Resolver>;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(VecResolver(self.as_slice().archive_ref(writer)?))
    }
}

impl<T: Deref<Target = [U]>, U: PartialEq<V>, V> PartialEq<Vec<V>> for ArchivedVec<T> {
    fn eq(&self, other: &Vec<V>) -> bool {
        self.deref().eq(other.deref())
    }
}

// TODO: impl Archive for HashMap/Set, etc
