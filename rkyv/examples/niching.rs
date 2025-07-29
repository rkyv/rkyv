use std::num::NonZeroU64;

use rancor::Failure;
use rkyv::{
    niche::niching::{Bool, NaN, Niching, Null, Zero},
    primitive::ArchivedU32,
    with::{AsBox, MapNiche, NicheInto},
    Archive, Place, Serialize,
};

// For archivable types containing options you can apply nichings to shrink the
// archived type's size and thus reduce the amount of serialized bytes.
#[expect(unused)]
#[derive(Archive)]
struct ContainsNiches {
    // rkyv provides a bunch of nichings such as `Zero` or `Null`.
    #[rkyv(with = NicheInto<Zero>)]
    non_zero: Option<NonZeroU64>,
    #[rkyv(with = NicheInto<Null>)]
    boxed: Option<Box<[u8]>>,
}

// Here's the same type but without the niches.
#[allow(unused)]
#[derive(Archive)]
struct WithoutNiches {
    non_zero: Option<NonZeroU64>,
    boxed: Option<Box<[u8]>>,
}

fn archived_size_check() {
    // Verify that niching really does reduce the archived size
    assert!(
        size_of::<ArchivedContainsNiches>()
            < size_of::<ArchivedWithoutNiches>()
    );
}

// Implement the `Niching` trait for custom nichings.
// Let's say we want a niching for `u32`s that never reach the max value.
struct Max;

impl Niching<ArchivedU32> for Max {
    unsafe fn is_niched(niched: *const ArchivedU32) -> bool {
        unsafe { (*niched).to_native() == u32::MAX }
    }

    fn resolve_niched(out: Place<ArchivedU32>) {
        out.write(ArchivedU32::from_native(u32::MAX));
    }
}

#[derive(Archive, Serialize)]
struct ContainsMaxNiche {
    #[rkyv(with = NicheInto<Max>)]
    int: Option<u32>,
}

fn use_max_niche() -> Result<(), Failure> {
    let x = ContainsMaxNiche { int: Some(3) };
    let bytes = rkyv::to_bytes(&x)?;
    let archived: &ArchivedContainsMaxNiche = rkyv::access(&bytes)?;
    assert_eq!(x.int.unwrap(), archived.int.as_ref().unwrap().to_native());

    // Note that this means if the max value would be reached after all,
    // the archived option will be considered as `None`.
    let y = ContainsMaxNiche {
        int: Some(u32::MAX),
    };
    let bytes = rkyv::to_bytes(&y)?;
    let archived: &ArchivedContainsMaxNiche = rkyv::access(&bytes)?;
    assert!(archived.int.is_none());

    Ok(())
}

// It is also possible to propagate nichings up into an outer type.
#[derive(Archive)]
struct Foo {
    // Annotating a field with `niche = ..` makes the outer type nichable.
    // More precisely, the annotations below implement `Niching<ArchivedFoo>`
    // for `Bool` and `NaN`.
    #[rkyv(niche = Bool)]
    boolean: bool,
    #[rkyv(niche = NaN)]
    float: f32,
}

#[expect(unused)]
#[derive(Archive)]
struct Bar {
    // Using the `Bool` niching to niche the `foo` field
    #[rkyv(with = NicheInto<Bool>)]
    // Here we make the outer type `Bar` nichable into `NaN`
    #[rkyv(niche = NaN)]
    foo: Option<Foo>,
}

// Lastly, a honorable mention to the `MapNiche` with-wrapper. It allows for
// neat optimizations such as:
#[derive(Archive, Serialize)]
struct NichedExample {
    // First applies the `AsBox` wrapper to archive into
    // `ArchivedBox<ArchivedHugeType>`, then applies the `Null` niching.
    #[rkyv(with = MapNiche<AsBox, Null>)]
    option: Option<HugeType>,
}

#[derive(Archive, Serialize)]
struct HugeType([u8; 1024]);

// Same type as above but without niching
#[derive(Archive, Serialize)]
struct BasicExample {
    option: Option<HugeType>,
}

fn map_niche() -> Result<(), Failure> {
    let basic = BasicExample { option: None };
    let bytes = rkyv::to_bytes(&basic)?;
    assert_eq!(bytes.len(), 1 + 1024); // full size despite being `None`

    let niched = NichedExample { option: None };
    let bytes = rkyv::to_bytes(&niched)?;
    assert_eq!(bytes.len(), 4); // size_of::<ArchivedBox<_>>()

    Ok(())
}

fn main() -> Result<(), Failure> {
    archived_size_check();
    use_max_niche()?;
    map_niche()?;

    Ok(())
}
