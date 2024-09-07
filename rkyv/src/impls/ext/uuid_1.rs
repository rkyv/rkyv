use rancor::Fallible;
use uuid_1::Uuid;

use crate::{
    traits::CopyOptimization, Archive, Deserialize, Place, Portable, Serialize,
};

// SAFETY: `Uuid` has the same ABI has `Bytes`, and so is `Portable` when
// `Bytes` is.
unsafe impl Portable for Uuid where uuid_1::Bytes: Portable {}

impl Archive for Uuid {
    const COPY_OPTIMIZATION: CopyOptimization<Self> =
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
mod tests {
    use super::Uuid;
    use crate::api::test::roundtrip;

    #[test]
    fn roundtrip_uuid() {
        roundtrip(
            &Uuid::parse_str("f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4").unwrap(),
        )
    }
}
