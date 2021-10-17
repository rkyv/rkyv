use crate::{Archive, Deserialize, Fallible, Serialize};
use rust_decimal::Decimal;

impl Archive for Decimal {
    type Archived = Decimal;
    type Resolver = ();

    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: *mut Self::Archived) {
        // Safety: 
        out.write(*self);
    }
}

// Safety: 
#[cfg(feature = "copy")]
unsafe impl crate::copy::ArchiveCopySafe for Decimal {}

impl<S: Fallible + ?Sized> Serialize<S> for Decimal {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Decimal, D> for Decimal {
    fn deserialize(&self, _: &mut D) -> Result<Decimal, D::Error> {
        Ok(*self)
    }
}

#[cfg(test)]
mod rkyv_tests {
    use crate::{
        archived_root,
        ser::{serializers::AlignedSerializer, Serializer},
        util::AlignedVec,
        Deserialize, Infallible,
    };
    use rust_decimal::Decimal;
    use std::str::FromStr;
    
    #[test]
    fn test_serialize_deserialize() {
        let amount = Decimal::from_str("25.12").unwrap();

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(&amount)
            .expect("failed to archive decimal");
        let buf = serializer.into_inner();
        let archived = unsafe { archived_root::<Decimal>(buf.as_ref()) };

        assert_eq!(&amount, archived);

        let deserialized = archived
            .deserialize(&mut Infallible)
            .expect("failed to deserialize decimal");

        assert_eq!(amount, deserialized);
    }
}
