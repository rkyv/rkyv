use rancor::Fallible;
use uuid::Uuid;

use crate::{
    Archive, CopyOptimization, Deserialize, Place, Portable, Serialize,
};

unsafe impl Portable for Uuid {}

impl Archive for Uuid {
    const COPY_OPTIMIZATION: crate::CopyOptimization<Self> =
        unsafe { CopyOptimization::enable() };

    type Archived = Uuid;
    type Resolver = ();

    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        // SAFETY: `Uuid` is guaranteed to have the same ABI as `[u8; 16]`,
        // which is always fully-initialized.
        unsafe {
            out.write_unchecked(*self);
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for Uuid {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Uuid, D> for Uuid {
    fn deserialize(&self, _: &mut D) -> Result<Uuid, D::Error> {
        Ok(*self)
    }
}

#[cfg(test)]
mod rkyv_tests {
    use uuid::Uuid;

    use crate::test::roundtrip;

    #[test]
    fn roundtrip_uuid() {
        roundtrip(
            &Uuid::parse_str("f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4").unwrap(),
        )
    }
}
