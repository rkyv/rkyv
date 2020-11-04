#![cfg_attr(feature = "const_generics", feature(const_generics))]
#![cfg_attr(feature = "const_generics", allow(incomplete_features))]

mod core_impls;
#[cfg(not(feature = "no_std"))]
mod std_impls;

use core::{
    marker::PhantomData,
    ops::Deref,
};

pub trait Write {
    type Error;

    fn pos(&self) -> usize;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        debug_assert!(align & (align - 1) == 0);

        let offset = self.pos() & (align - 1);
        if offset != 0 {
            const ZEROES_LEN: usize = 16;
            const ZEROES: [u8; ZEROES_LEN] = [0; ZEROES_LEN];

            let mut padding = align - offset;
            loop {
                let len = usize::min(ZEROES_LEN, padding);
                self.write(&ZEROES[0..len])?;
                padding -= len;
                if padding == 0 {
                    break;
                }
            }
        }
        Ok(self.pos())
    }

    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
        self.align(core::mem::align_of::<T>())
    }

    // This is only safe to call when the writer is already aligned for an Archived<T>
    unsafe fn resolve_aligned<T: ?Sized, R: Resolve<T>>(&mut self, value: &T, resolver: R) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert!(pos & (core::mem::align_of::<R::Archived>() - 1) == 0);
        let archived = &resolver.resolve(pos, value);
        let data = (archived as *const R::Archived).cast::<u8>();
        let len = core::mem::size_of::<R::Archived>();
        self.write(core::slice::from_raw_parts(data, len))?;
        Ok(pos)
    }

    fn archive<T: Archive>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.archive(self)?;
        self.align_for::<T::Archived>()?;
        unsafe {
            self.resolve_aligned(value, resolver)
        }
    }

    fn archive_ref<T: ArchiveRef + ?Sized>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.archive_ref(self)?;
        self.align_for::<T::Reference>()?;
        unsafe {
            self.resolve_aligned(value, resolver)
        }
    }
}

pub trait Resolve<T: ?Sized> {
    type Archived;

    fn resolve(self, pos: usize, value: &T) -> Self::Archived;
}

pub trait Archive {
    type Archived;
    type Resolver: Resolve<Self, Archived = Self::Archived>;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error>;
}

pub trait ArchiveRef {
    type Archived: ?Sized;
    type Reference: Deref<Target = Self::Archived>;
    type Resolver: Resolve<Self, Archived = Self::Reference>;

    fn archive_ref<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error>;
}

pub struct Identity;

impl<T: Archive + Copy> Resolve<T> for Identity {
    type Archived = T;

    fn resolve(self, _pos: usize, value: &T) -> Self::Archived {
        *value
    }
}

#[repr(transparent)]
pub struct RelPtr<T> {
    offset: i32,
    _phantom: PhantomData<T>,
}

impl<T> RelPtr<T> {
    pub fn new(from: usize, to: usize) -> Self {
        Self {
            offset: (to as isize - from as isize) as i32,
            _phantom: PhantomData,
        }
    }

    pub fn as_ptr(&self) -> *const T {
        unsafe {
            (self as *const Self).cast::<u8>().offset(self.offset as isize).cast::<T>()
        }
    }
}

impl<T> Deref for RelPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_ptr() }
    }
}

impl<T: Archive> Resolve<T> for usize {
    type Archived = RelPtr<T::Archived>;

    fn resolve(self, pos: usize, _value: &T) -> Self::Archived {
        RelPtr::new(pos, self)
    }
}

#[macro_export]
macro_rules! rel_ptr {
    ($from:expr, $to:expr, $parent:path, $field:ident) => (
        RelPtr::new($from + memoffset::offset_of!($parent, $field), $to)
    )
}

impl<T: Archive> ArchiveRef for T {
    type Archived = <T::Resolver as Resolve<T>>::Archived;
    type Reference = RelPtr<Self::Archived>;
    type Resolver = usize;

    fn archive_ref<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(writer.archive(self)?)
    }
}

#[cfg(not(feature = "no_std"))]
pub struct ArchiveWriter<W: std::io::Write> {
    inner: W,
    pos: usize,
}

#[cfg(not(feature = "no_std"))]
impl<W: std::io::Write> ArchiveWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            pos: 0,
        }
    }

    pub fn with_pos(inner: W, pos: usize) -> Self {
        Self {
            inner,
            pos,
        }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[cfg(not(feature = "no_std"))]
impl<W: std::io::Write> Write for ArchiveWriter<W> {
    type Error = std::io::Error;

    fn pos(&self) -> usize {
        self.pos
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        println!("Wrote {} bytes: {:?}", bytes.len(), bytes);
        self.pos += self.inner.write(bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::ops::Deref;
    use crate::{
        Archive,
        ArchiveRef,
        ArchiveWriter,
        Write,
    };

    fn test_archive<T: Archive<Archived = U> + PartialEq<U>, U>(value: &T) {
        let mut writer = ArchiveWriter::new(Vec::new());
        let pos = writer.archive(value).expect("failed to archive value");
        let buf = writer.into_inner();
        let archived_value = unsafe { &*buf.as_ptr().offset(pos as isize).cast::<U>() };
        assert!(value.eq(archived_value));
    }

    fn test_archive_ref<T: ArchiveRef<Archived = U> + PartialEq<U> + ?Sized, U: ?Sized>(value: &T) {
        let mut writer = ArchiveWriter::new(Vec::new());
        let pos = writer.archive_ref(value).expect("failed to archive ref");
        let buf = writer.into_inner();
        let archived_ref = unsafe { &*buf.as_ptr().offset(pos as isize).cast::<T::Reference>() };
        assert!(value.eq(archived_ref));
    }

    fn test_archive_container<T: Archive<Archived = U> + Deref<Target = TV>, TV: PartialEq<TU> + ?Sized, U: Deref<Target = TU>, TU: ?Sized>(value: &T) {
        let mut writer = ArchiveWriter::new(Vec::new());
        let pos = writer.archive(value).expect("failed to archive ref");
        let buf = writer.into_inner();
        let archived_ref = unsafe { &*buf.as_ptr().offset(pos as isize).cast::<U>() };
        assert!(value.eq(archived_ref));
    }

    #[test]
    fn archive_primitives() {
        test_archive(&());
        test_archive(&true);
        test_archive(&false);
        test_archive(&1234567f32);
        test_archive(&12345678901234f64);
        test_archive(&123i8);
        test_archive(&123456i32);
        test_archive(&1234567890i128);
        test_archive(&123u8);
        test_archive(&123456u32);
        test_archive(&1234567890u128);
        test_archive(&(24, true, 16f32));
        test_archive(&[1, 2, 3, 4, 5, 6]);

        test_archive(&Option::<()>::None);
        test_archive(&Some(42));
    }

    #[test]
    fn archive_refs() {
        test_archive_ref::<[i32; 4], _>(&[1, 2, 3, 4]);
        test_archive_ref::<str, _>("hello world");
        test_archive_ref::<[i32], _>([1, 2, 3, 4].as_ref());
    }

    #[test]
    fn archive_containers() {
        test_archive_container(&Box::new(42));
        test_archive_container(&"hello world".to_string().into_boxed_str());
        test_archive_container(&vec![1, 2, 3, 4].into_boxed_slice());
        test_archive_container(&"hello world".to_string());
        test_archive_container(&vec![1, 2, 3, 4]);
    }

    #[test]
    fn archive_composition() {
        test_archive(&Some(Box::new(42)));
        test_archive(&Some("hello world".to_string().into_boxed_str()));
        test_archive(&Some(vec![1, 2, 3, 4].into_boxed_slice()));
        test_archive(&Some("hello world".to_string()));
        test_archive(&Some(vec![1, 2, 3, 4]));
        test_archive(&Some(Box::new(vec![1, 2, 3, 4])));
    }
}
