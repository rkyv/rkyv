use crate::{
    Archive,
    Archived,
    rel_ptr,
    RelativePointer,
    Resolve,
    Resolver,
    Write,
};

impl<T: Archive> Resolve<Box<T>> for usize {
    type Archived = RelativePointer<Archived<T>>;

    fn resolve(self, pos: usize, _value: &Box<T>) -> Self::Archived {
        RelativePointer::new((self as isize - pos as isize) as i32)
    }
}

impl<T: Archive> Archive for Box<T> {
    type Resolver = usize;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        writer.archive(self.as_ref())
    }
}

pub struct ArchivedSlice<T> {
    ptr: RelativePointer<T>,
    len: u32,
}

impl<T> ArchivedSlice<T> {
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe {
            core::slice::from_raw_parts(self.as_ptr(), self.len as usize)
        }
    }
}

impl<T: Archive> Resolve<[T]> for usize
where
    [T]: Archive
{
    type Archived = ArchivedSlice<Archived<T>>;

    fn resolve(self, pos: usize, value: &[T]) -> Self::Archived {
        ArchivedSlice {
            ptr: rel_ptr!(pos, Self::Archived, ptr, self),
            len: value.len() as u32,
        }
    }
}

impl<T: Archive> Archive for [T] {
    type Resolver = usize;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        let mut resolvers = Vec::with_capacity(self.len());
        for i in 0..self.len() {
            resolvers.push(self[i].archive(writer)?);
        }
        let result = writer.align_for::<Archived<T>>()?;
        unsafe {
            for (i, resolver) in resolvers.drain(..).enumerate() {
                writer.resolve_aligned(&self[i], resolver)?;
            }
        }
        Ok(result)
    }
}

impl<T: Archive> Resolve<Vec<T>> for Resolver<[T]>
where
    Vec<T>: Archive
{
    type Archived = Archived<[T]>;

    fn resolve(self, pos: usize, value: &Vec<T>) -> Self::Archived {
        self.resolve(pos, value.as_slice())
    }
}

impl<T: Archive> Archive for Vec<T> {
    type Resolver = Resolver<[T]>;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        self.as_slice().archive(writer)
    }
}

// TODO: impl Archive for HashMap/Set, etc
