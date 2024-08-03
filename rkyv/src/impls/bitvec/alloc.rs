use bitvec::prelude::*;
use munge::munge;
use rancor::{Fallible, Source};

use crate::{
    bitvec::ArchivedBitVec,
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    Archive, Archived, Deserialize, Place, Serialize,
};

impl<T: BitStore + Archive, O: BitOrder> Archive for BitVec<T, O>
where
    Archived<T>: BitStore,
{
    type Archived = ArchivedBitVec<Archived<T>, O>;
    type Resolver = VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let ArchivedBitVec { inner, bit_len, _or: _ } = out);
        ArchivedVec::resolve_from_slice(self.as_raw_slice(), resolver, inner);
        usize::resolve(&self.len(), (), bit_len);
    }
}

impl<T, O, S> Serialize<S> for BitVec<T, O>
where
    T: BitStore + Archive + Serialize<S>,
    O: BitOrder,
    S: Fallible + ?Sized + Allocator + Writer,
    Archived<T>: BitStore,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        let resolver =
            ArchivedVec::serialize_from_slice(self.as_raw_slice(), serializer)?;
        usize::serialize(&self.len(), serializer)?;

        Ok(resolver)
    }
}

impl<T, O, D> Deserialize<BitVec<T, O>, D> for ArchivedBitVec<Archived<T>, O>
where
    T: BitStore + Archive,
    O: BitOrder,
    D: Fallible + ?Sized,
    D::Error: Source,
    Archived<T>: Deserialize<T, D> + BitStore,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<BitVec<T, O>, <D as Fallible>::Error> {
        let vec = ArchivedVec::deserialize(&self.inner, deserializer)?;
        let bit_len =
            Archived::<usize>::deserialize(&self.bit_len, deserializer)?;

        let mut bitvec = BitVec::<T, O>::from_vec(vec);
        bitvec.truncate(bit_len);
        Ok(bitvec)
    }
}

// TODO: needs rend to have bitvec support
// #[cfg(test)]
// mod tests {
//     use crate::{
//         archived_root,
//         ser::{serializers::CoreSerializer, Serializer},
//         Deserialize,
//     };
//     use bitvec::prelude::*;
//     use rancor::{Strategy, Infallible};

//     #[test]
//     #[cfg(all(feature = "bitvec", feature = "alloc"))]
//     fn bitvec() {
//         use rancor::{Infallible, Strategy};

//         use crate::ser::serializers::CoreSerializer;

//         let original = bitvec![1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1];

//         let serializer = crate::to_bytes_with(
//             &original,
//             CoreSerializer::<256, 256>::default(),
//         ).unwrap();
//         let end = serializer.pos();
//         let buffer = serializer.into_serializer().into_inner();

//         let output = unsafe { archived_root::<BitVec>(&buffer[0..end]) };
//         assert_eq!(&original, output.as_bitslice());

//         let deserialized = deserialize::<BitVec, _, Infallible>(output, &mut
// ()).unwrap();         assert_eq!(deserialized, original);
//     }
// }
