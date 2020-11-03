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
    unsafe fn resolve_aligned<T: Archive + ?Sized, R: Resolve<T>>(&mut self, value: &T, resolver: R) -> Result<usize, Self::Error> {
        let pos = self.pos();
        debug_assert!(pos & (core::mem::align_of::<Archived<T>>() - 1) == 0);
        let archived = &resolver.resolve(pos, value);
        let data = (archived as *const R::Archived).cast::<u8>();
        let len = core::mem::size_of::<R::Archived>();
        self.write(core::slice::from_raw_parts(data, len))?;
        Ok(pos)
    }

    fn archive<T: Archive + ?Sized>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let resolver = value.archive(self)?;
        self.align_for::<Archived<T>>()?;
        unsafe {
            self.resolve_aligned(value, resolver)
        }
    }
}

pub trait Resolve<T: Archive + ?Sized> {
    type Archived;

    fn resolve(self, pos: usize, value: &T) -> Self::Archived;
}

pub trait Archive {
    type Resolver: Resolve<Self>;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error>;
}

pub type Resolver<T> = <T as Archive>::Resolver;
pub type Archived<T> = <<T as Archive>::Resolver as Resolve<T>>::Archived;

pub struct Identity<T> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T> Identity<T> {
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T: Archive + Copy> Resolve<T> for Identity<T> {
    type Archived = T;

    fn resolve(self, _pos: usize, value: &T) -> Self::Archived {
        *value
    }
}

#[repr(transparent)]
pub struct RelativePointer<T> {
    offset: i32,
    _phantom: PhantomData<T>,
}

impl<T> RelativePointer<T> {
    pub fn new(offset: i32) -> Self {
        Self {
            offset,
            _phantom: PhantomData,
        }
    }

    pub fn as_ptr(&self) -> *const T {
        unsafe {
            (self as *const Self).cast::<u8>().offset(self.offset as isize).cast::<T>()
        }
    }
}

impl<T> Deref for RelativePointer<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_ptr() }
    }
}

#[macro_export]
macro_rules! rel_ptr {
    ($base:expr, $parent:path, $field:ident, $dest:expr) => (
        RelativePointer::new(($dest as isize - $base as isize + memoffset::offset_of!($parent, $field) as isize) as i32)
    )
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
}

#[cfg(not(feature = "no_std"))]
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
    // TODO: write tests and make sure nothing is broken
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
