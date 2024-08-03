use core::ops::Deref;

use bitvec::{prelude::*, view::BitViewSized};
use munge::munge;
use rancor::Fallible;

#[cfg(feature = "alloc")]
mod alloc;

use crate::{
    bitvec::{ArchivedBitArray, ArchivedBitVec},
    ser::{Allocator, Writer},
    Archive, Archived, Deserialize, Place, Serialize,
};

impl<T: BitStore + Archive, O: BitOrder> ArchivedBitVec<T, O> {
    /// Gets the elements of the archived `BitVec` as a `BitSlice`.
    pub fn as_bitslice(&self) -> &BitSlice<T, O> {
        self.deref()
    }
}

impl<A: BitViewSized + Archive, O: BitOrder> Archive for BitArray<A, O>
where
    Archived<A>: BitViewSized,
    for<'a> &'a A: TryFrom<&'a [A::Store]>,
{
    type Archived = ArchivedBitArray<Archived<A>, O>;
    type Resolver = A::Resolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        let arr_ref = self.as_raw_slice().try_into().ok().unwrap();

        munge!(let ArchivedBitArray { inner, _or: _ } = out);
        A::resolve(arr_ref, resolver, inner);
    }
}

impl<A, O, S> Serialize<S> for BitArray<A, O>
where
    A: BitViewSized + Archive + Serialize<S>,
    O: BitOrder,
    S: Fallible + ?Sized + Allocator + Writer,
    Archived<A>: BitViewSized,
    for<'a> &'a A: TryFrom<&'a [A::Store]>,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        let arr_ref = self.as_raw_slice().try_into().ok().unwrap();
        let resolver = A::serialize(arr_ref, serializer)?;

        Ok(resolver)
    }
}

impl<A, O, D> Deserialize<BitArray<A, O>, D>
    for ArchivedBitArray<Archived<A>, O>
where
    A: BitViewSized + Archive,
    O: BitOrder,
    D: Fallible + ?Sized,
    A::Archived: Deserialize<A, D> + BitViewSized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<BitArray<A, O>, <D as Fallible>::Error> {
        let arr = Archived::<A>::deserialize(&self.inner, deserializer)?;
        Ok(arr.into())
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
//     fn bitarr() {
//         let original = bitarr![1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1];

//         let serializer = crate::to_bytes_with(
//             &original,
//             CoreSerializer::<256, 256>::default(),
//         ).unwrap();
//         let end = serializer.pos();
//         let buffer = serializer.into_serializer().into_inner();

//         let output = unsafe { archived_root::<BitArray>(&buffer[0..end]) };
//         assert_eq!(&original[..11], &output[..11]);

//         let deserialized = deserialize::<BitArray, _, Infallible>(output,
// &mut ()).unwrap();         assert_eq!(&deserialized[..11], &original[..11]);
//     }
// }
