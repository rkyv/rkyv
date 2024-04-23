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
    use rancor::Infallible;
    use uuid::Uuid;

    use crate::{access_unchecked, deserialize, util::AlignedVec};

    #[test]
    fn test_serialize_deserialize() {
        let uuid_str = "f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4";
        let u = Uuid::parse_str(uuid_str).unwrap();

        let buf =
            crate::util::serialize_into::<_, Infallible>(&u, AlignedVec::new())
                .expect("failed to archive uuid");
        let archived = unsafe { access_unchecked::<Uuid>(buf.as_ref()) };

        assert_eq!(&u, archived);

        let deserialized =
            deserialize::<Uuid, _, Infallible>(archived, &mut ())
                .expect("failed to deserialize uuid");

        assert_eq!(u, deserialized);
    }
}
