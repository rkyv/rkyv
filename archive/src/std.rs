use std::io::Write;
use crate::{
    Archive,
    Result,
};

pub struct ArchiveWriter<W: Write> {
    inner: W,
}

impl<W: Write> ArchiveWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
        }
    }

    pub fn write_object<T: Archive>(&mut self, object: T) -> Result<()> {
        let archived = object.archive();
        let data = (&archived as *const T::Archived).cast::<u8>();
        let len = core::mem::size_of::<T::Archived>();
        unsafe {
            self.inner.write(core::slice::from_raw_parts(data, len))?;
        }
        Ok(())
    }
}

// TODO: impl Archive for Box, Vec, HashMap/Set, etc
