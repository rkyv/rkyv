#![cfg(all(feature = "bytecheck", feature = "alloc"))]

use std::{
    rc::{Rc, Weak},
    sync::Arc,
};

use rkyv::{Archive, Deserialize, Place, Serialize};

#[derive(Debug)]
struct Thin(u8);

#[repr(align(64))]
#[derive(Debug)]
struct Wide(u8);

impl Archive for Thin {
    type Archived = u8;
    type Resolver = ();

    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        out.write(self.0);
    }
}

impl<S: rancor::Fallible + ?Sized> Serialize<S> for Thin {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: rancor::Fallible + ?Sized> Deserialize<Thin, D> for u8 {
    fn deserialize(&self, _: &mut D) -> Result<Thin, D::Error> {
        Ok(Thin(*self))
    }
}

impl Archive for Wide {
    type Archived = u8;
    type Resolver = ();

    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        out.write(self.0);
    }
}

impl<S: rancor::Fallible + ?Sized> Serialize<S> for Wide {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: rancor::Fallible + ?Sized> Deserialize<Wide, D> for u8 {
    fn deserialize(&self, _: &mut D) -> Result<Wide, D::Error> {
        Ok(Wide(*self))
    }
}

#[cfg(feature = "pointer_width_16")]
fn offset_to_bytes(offset: rkyv::primitive::FixedIsize) -> [u8; 2] {
    #[cfg(feature = "big_endian")]
    {
        offset.to_be_bytes()
    }
    #[cfg(not(feature = "big_endian"))]
    {
        offset.to_le_bytes()
    }
}

#[cfg(not(any(feature = "pointer_width_16", feature = "pointer_width_64")))]
fn offset_to_bytes(offset: rkyv::primitive::FixedIsize) -> [u8; 4] {
    #[cfg(feature = "big_endian")]
    {
        offset.to_be_bytes()
    }
    #[cfg(not(feature = "big_endian"))]
    {
        offset.to_le_bytes()
    }
}

#[cfg(feature = "pointer_width_64")]
fn offset_to_bytes(offset: rkyv::primitive::FixedIsize) -> [u8; 8] {
    #[cfg(feature = "big_endian")]
    {
        offset.to_be_bytes()
    }
    #[cfg(not(feature = "big_endian"))]
    {
        offset.to_le_bytes()
    }
}

#[cfg(feature = "pointer_width_16")]
fn offset_from_bytes(bytes: &[u8]) -> rkyv::primitive::FixedIsize {
    let bytes = bytes[..2].try_into().unwrap();
    #[cfg(feature = "big_endian")]
    {
        i16::from_be_bytes(bytes)
    }
    #[cfg(not(feature = "big_endian"))]
    {
        i16::from_le_bytes(bytes)
    }
}

#[cfg(not(any(feature = "pointer_width_16", feature = "pointer_width_64")))]
fn offset_from_bytes(bytes: &[u8]) -> rkyv::primitive::FixedIsize {
    let bytes = bytes[..4].try_into().unwrap();
    #[cfg(feature = "big_endian")]
    {
        i32::from_be_bytes(bytes)
    }
    #[cfg(not(feature = "big_endian"))]
    {
        i32::from_le_bytes(bytes)
    }
}

#[cfg(feature = "pointer_width_64")]
fn offset_from_bytes(bytes: &[u8]) -> rkyv::primitive::FixedIsize {
    let bytes = bytes[..8].try_into().unwrap();
    #[cfg(feature = "big_endian")]
    {
        i64::from_be_bytes(bytes)
    }
    #[cfg(not(feature = "big_endian"))]
    {
        i64::from_le_bytes(bytes)
    }
}

fn point_rel_ptr_at(bytes: &mut [u8], rel_ptr_offset: usize, target: usize) {
    let rel_ptr_addr = bytes.as_ptr() as usize + rel_ptr_offset;
    let offset =
        rkyv::rel_ptr::signed_offset::<rancor::Error>(rel_ptr_addr, target)
            .unwrap();
    let offset: rkyv::primitive::FixedIsize = offset.try_into().unwrap();
    let offset_bytes = offset_to_bytes(offset);

    bytes[rel_ptr_offset..rel_ptr_offset + offset_bytes.len()]
        .copy_from_slice(&offset_bytes);
}

fn rel_ptr_target(bytes: &[u8], rel_ptr_offset: usize) -> usize {
    let offset = offset_from_bytes(&bytes[rel_ptr_offset..]) as isize;
    (bytes.as_ptr() as usize + rel_ptr_offset).wrapping_add(offset as usize)
}

#[test]
fn checked_rc_deserialization_reuses_pool_entry_as_wrong_type() {
    #[derive(Archive, Deserialize, Serialize)]
    struct Test {
        thin: Rc<Thin>,
        wide: Rc<Wide>,
    }

    // Issue: shared-pointer validation keys an already-validated pointee by
    // archived pointee type (`u8` here), flavor, address, and metadata. It does
    // not include the Rust target type that deserialization will later
    // allocate. Checked validation therefore accepts both fields pointing to
    // the same archived `u8`. During checked deserialization, the pool is keyed
    // only by archived address, so the `Rc<Wide>` field reuses the allocation
    // that was created as `Rc<Thin>`.
    let value = Test {
        thin: Rc::new(Thin(11)),
        wide: Rc::new(Wide(22)),
    };

    // Start from a completely valid archive. The only trick in this PoC is
    // that `Thin` and `Wide` both archive to the same type (`u8`), even though
    // deserializing them allocates Rust values with different layouts.
    let mut bytes = rkyv::to_bytes::<rancor::Error>(&value).unwrap();

    let (thin_pointee, wide_rel_ptr_offset) = {
        // This checked access is still looking at the original, valid archive.
        // It is used only to find the byte offsets of the two archived
        // `ArchivedRc<u8, RcFlavor>` fields.
        let archived =
            rkyv::access::<ArchivedTest, rancor::Error>(&bytes).unwrap();

        // `ArchivedRc` is represented by a relative pointer. For these sized
        // `u8` pointees, the archived field starts with the relative pointer
        // offset itself. So the field address is also the byte offset we need
        // in order to inspect or rewrite that pointer in the backing buffer.
        let thin_rel_ptr_offset = (&archived.thin as *const _ as *const u8
            as usize)
            - bytes.as_ptr() as usize;
        let wide_rel_ptr_offset = (&archived.wide as *const _ as *const u8
            as usize)
            - bytes.as_ptr() as usize;

        // Compute where the `thin` relative pointer currently points, then
        // remember where the `wide` relative pointer is stored. We will make
        // `wide` point at the exact same archived `u8` as `thin`.
        (
            rel_ptr_target(&bytes, thin_rel_ptr_offset),
            wide_rel_ptr_offset,
        )
    };

    // Forge the archive: `thin` is still an `Rc<Thin>`, but the archived
    // `wide: Rc<Wide>` now points to the same archived `u8` as `thin`.
    // Nothing unsafe is needed to create these bytes; this is just editing the
    // serialized relative pointer.
    point_rel_ptr_at(&mut bytes, wide_rel_ptr_offset, thin_pointee);

    // This is the call under test. Checked validation accepts the forged
    // archive because both shared pointers have the same archived pointee type
    // (`u8`) and flavor. During deserialization, the shared-pointer pool is
    // keyed by archived address, so `Rc<Wide>` tries to reuse the allocation
    // that was created for `Rc<Thin>`. The pool must reject that type mismatch
    // before `Rc::<Wide>::increment_strong_count` is called.
    assert!(rkyv::from_bytes::<Test, rancor::Error>(&bytes).is_err());
}

#[cfg(target_has_atomic = "ptr")]
#[test]
fn checked_arc_deserialization_reuses_pool_entry_as_wrong_type() {
    #[derive(Archive, Deserialize, Serialize)]
    struct Test {
        thin: Arc<Thin>,
        wide: Arc<Wide>,
    }

    let value = Test {
        thin: Arc::new(Thin(11)),
        wide: Arc::new(Wide(22)),
    };
    let mut bytes = rkyv::to_bytes::<rancor::Error>(&value).unwrap();

    let (thin_pointee, wide_rel_ptr_offset) = {
        let archived =
            rkyv::access::<ArchivedTest, rancor::Error>(&bytes).unwrap();
        let thin_rel_ptr_offset = (&archived.thin as *const _ as *const u8
            as usize)
            - bytes.as_ptr() as usize;
        let wide_rel_ptr_offset = (&archived.wide as *const _ as *const u8
            as usize)
            - bytes.as_ptr() as usize;
        (
            rel_ptr_target(&bytes, thin_rel_ptr_offset),
            wide_rel_ptr_offset,
        )
    };
    point_rel_ptr_at(&mut bytes, wide_rel_ptr_offset, thin_pointee);

    assert!(rkyv::from_bytes::<Test, rancor::Error>(&bytes).is_err());
}

#[test]
fn checked_weak_deserialization_reuses_pool_entry_as_wrong_type() {
    #[derive(Archive, Deserialize, Serialize)]
    struct Test {
        thin: Rc<Thin>,
        weak: Weak<Wide>,
    }

    let wide = Rc::new(Wide(22));
    let value = Test {
        thin: Rc::new(Thin(11)),
        weak: Rc::downgrade(&wide),
    };
    let mut bytes = rkyv::to_bytes::<rancor::Error>(&value).unwrap();

    let (thin_pointee, weak_rel_ptr_offset) = {
        let archived =
            rkyv::access::<ArchivedTest, rancor::Error>(&bytes).unwrap();
        let thin_rel_ptr_offset = (&archived.thin as *const _ as *const u8
            as usize)
            - bytes.as_ptr() as usize;
        let weak_rel_ptr_offset = (&archived.weak as *const _ as *const u8
            as usize)
            - bytes.as_ptr() as usize;
        (
            rel_ptr_target(&bytes, thin_rel_ptr_offset),
            weak_rel_ptr_offset,
        )
    };
    point_rel_ptr_at(&mut bytes, weak_rel_ptr_offset, thin_pointee);

    assert!(rkyv::from_bytes::<Test, rancor::Error>(&bytes).is_err());
}
