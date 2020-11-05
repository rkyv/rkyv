#![cfg_attr(any(feature = "const_generics", feature = "specialization"), allow(incomplete_features))]
#![cfg_attr(feature = "const_generics", feature(const_generics))]
#![cfg_attr(feature = "specialization", feature(specialization))]

mod builtin;

use core::{
    hash::{
        Hash,
        Hasher,
    },
    marker::PhantomData,
    ops::Deref,
};
pub use memoffset::offset_of;

#[cfg(feature = "specialization")]
#[macro_export]
macro_rules! default {
    ($($rest:tt)*) => { default $($rest)* };
}

#[cfg(not(feature = "specialization"))]
#[macro_export]
macro_rules! default {
    ($($rest:tt)*) => { $($rest)* };
}

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

impl<T: Archive<Archived = T> + Copy> Resolve<T> for () {
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

impl<T: Hash> Hash for RelPtr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<T: PartialEq> PartialEq for RelPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: Eq> Eq for RelPtr<T> {}

impl<T: Archive> Resolve<T> for usize {
    type Archived = RelPtr<T::Archived>;

    fn resolve(self, pos: usize, _value: &T) -> Self::Archived {
        RelPtr::new(pos, self)
    }
}

impl<T: Archive> ArchiveRef for T {
    type Archived = <T::Resolver as Resolve<T>>::Archived;
    type Reference = RelPtr<Self::Archived>;
    type Resolver = usize;

    fn archive_ref<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(writer.archive(self)?)
    }
}

pub struct ArchiveBuffer<T> {
    inner: T,
    pos: usize,
}

impl<T> ArchiveBuffer<T> {
    pub fn new(inner: T) -> Self {
        Self::with_pos(inner, 0)
    }

    pub fn with_pos(inner: T, pos: usize) -> Self {
        Self {
            inner,
            pos,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[derive(Debug)]
pub enum ArchiveBufferError {
    Overflow,
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Write for ArchiveBuffer<T> {
    type Error = ArchiveBufferError;

    fn pos(&self) -> usize {
        self.pos
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let end_pos = self.pos + bytes.len();
        if end_pos > self.inner.as_ref().len() {
            Err(ArchiveBufferError::Overflow)
        } else {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    self.inner.as_mut().as_mut_ptr().offset(self.pos as isize),
                    bytes.len());
            }
            self.pos = end_pos;
            Ok(())
        }
    }
}

#[cfg(feature = "std")]
pub struct ArchiveWriter<W: std::io::Write> {
    inner: W,
    pos: usize,
}

#[cfg(feature = "std")]
impl<W: std::io::Write> ArchiveWriter<W> {
    pub fn new(inner: W) -> Self {
        Self::with_pos(inner, 0)
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

#[cfg(feature = "std")]
impl<W: std::io::Write> Write for ArchiveWriter<W> {
    type Error = std::io::Error;

    fn pos(&self) -> usize {
        self.pos
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.pos += self.inner.write(bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Archive,
        ArchiveBuffer,
        ArchiveRef,
        Write,
    };

    #[repr(align(16))]
    struct Aligned<T>(T);

    impl<T: AsRef<[U]>, U> AsRef<[U]> for Aligned<T> {
        fn as_ref(&self) -> &[U] {
            self.0.as_ref()
        }
    }

    impl<T: AsMut<[U]>, U> AsMut<[U]> for Aligned<T> {
        fn as_mut(&mut self) -> &mut [U] {
            self.0.as_mut()
        }
    }

    const BUFFER_SIZE: usize = 256;

    fn test_archive<T: Archive<Archived = U> + PartialEq<U>, U>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(value).expect("failed to archive value");
        let buf = writer.into_inner();
        let archived_value = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<U>() };
        assert!(value.eq(archived_value));
    }

    fn test_archive_ref<T: ArchiveRef<Archived = U> + PartialEq<U> + ?Sized, U: ?Sized>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive_ref(value).expect("failed to archive ref");
        let buf = writer.into_inner();
        let archived_ref = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<T::Reference>() };
        assert!(value.eq(archived_ref));
    }

    #[cfg(feature = "std")]
    fn test_archive_container<T: Archive<Archived = U> + core::ops::Deref<Target = TV>, TV: PartialEq<TU> + ?Sized, U: core::ops::Deref<Target = TU>, TU: ?Sized>(value: &T) {
        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(value).expect("failed to archive ref");
        let buf = writer.into_inner();
        let archived_ref = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<U>() };
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

    #[cfg(feature = "std")]
    #[test]
    fn archive_containers() {
        test_archive_container(&Box::new(42));
        test_archive_container(&"hello world".to_string().into_boxed_str());
        test_archive_container(&vec![1, 2, 3, 4].into_boxed_slice());
        test_archive_container(&"hello world".to_string());
        test_archive_container(&vec![1, 2, 3, 4]);
    }

    #[cfg(feature = "std")]
    #[test]
    fn archive_composition() {
        test_archive(&Some(Box::new(42)));
        test_archive(&Some("hello world".to_string().into_boxed_str()));
        test_archive(&Some(vec![1, 2, 3, 4].into_boxed_slice()));
        test_archive(&Some("hello world".to_string()));
        test_archive(&Some(vec![1, 2, 3, 4]));
        test_archive(&Some(Box::new(vec![1, 2, 3, 4])));
    }

    #[cfg(feature = "std")]
    #[test]
    fn archive_hash_map() {
        use std::collections::HashMap;

        test_archive(&HashMap::<i32, i32>::new());

        let mut hash_map = HashMap::new();
        hash_map.insert(1, 2);
        hash_map.insert(3, 4);
        hash_map.insert(5, 6);
        hash_map.insert(7, 8);

        test_archive(&hash_map);

        let mut hash_map = HashMap::new();
        hash_map.insert("hello".to_string(), "world".to_string());
        hash_map.insert("foo".to_string(), "bar".to_string());
        hash_map.insert("baz".to_string(), "bat".to_string());

        let mut writer = ArchiveBuffer::new(Aligned([0u8; BUFFER_SIZE]));
        let pos = writer.archive(&hash_map).expect("failed to archive value");
        let buf = writer.into_inner();
        let archived_value = unsafe { &*buf.as_ref().as_ptr().offset(pos as isize).cast::<<HashMap<String, String> as Archive>::Archived>() };

        assert!(archived_value.len() == hash_map.len());

        for (key, value) in hash_map.iter() {
            assert!(archived_value.contains_key(key.as_str()));
            assert!(archived_value[key.as_str()].eq(value));
        }

        for (key, value) in archived_value.iter() {
            assert!(hash_map.contains_key(key.as_str()));
            assert!(hash_map[key.as_str()].eq(value));
        }
    }
}
