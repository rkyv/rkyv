use core::{
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};

use hashbrown::HashMap;
use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::{ArchivedHashMap, HashMapResolver},
    impls::core::with::RefWrapper,
    ser::{Allocator, Writer},
    with::{ArchiveWith, DeserializeWith, MapKV, SerializeWith},
    Place,
};

impl<A, B, K, V, H> ArchiveWith<HashMap<K, V, H>> for MapKV<A, B>
where
    A: ArchiveWith<K>,
    B: ArchiveWith<V>,
    H: Default + BuildHasher,
{
    type Archived = ArchivedHashMap<
        <A as ArchiveWith<K>>::Archived,
        <B as ArchiveWith<V>>::Archived,
    >;
    type Resolver = HashMapResolver;

    fn resolve_with(
        field: &HashMap<K, V, H>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedHashMap::resolve_from_len(field.len(), (7, 8), resolver, out)
    }
}

impl<A, B, K, V, H, S> SerializeWith<HashMap<K, V, H>, S> for MapKV<A, B>
where
    A: ArchiveWith<K> + SerializeWith<K, S>,
    B: ArchiveWith<V> + SerializeWith<V, S>,
    K: Hash + Eq,
    <A as ArchiveWith<K>>::Archived: Eq + Hash,
    S: Fallible + Writer + Allocator + ?Sized,
    S::Error: Source,
    H: Default + BuildHasher,
    H::Hasher: Default,
{
    fn serialize_with(
        field: &HashMap<K, V, H>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedHashMap::<_, _, H::Hasher>::serialize_from_iter(
            field.iter().map(|(k, v)| {
                (
                    RefWrapper::<'_, A, K>(k, PhantomData::<A>),
                    RefWrapper::<'_, B, V>(v, PhantomData::<B>),
                )
            }),
            (7, 8),
            serializer,
        )
    }
}

impl<A, B, K, V, D, S>
    DeserializeWith<
        ArchivedHashMap<
            <A as ArchiveWith<K>>::Archived,
            <B as ArchiveWith<V>>::Archived,
        >,
        HashMap<K, V, S>,
        D,
    > for MapKV<A, B>
where
    A: ArchiveWith<K> + DeserializeWith<<A as ArchiveWith<K>>::Archived, K, D>,
    B: ArchiveWith<V> + DeserializeWith<<B as ArchiveWith<V>>::Archived, V, D>,
    K: Ord + Hash + Eq,
    D: Fallible + ?Sized,
    S: Default + BuildHasher,
{
    fn deserialize_with(
        field: &ArchivedHashMap<
            <A as ArchiveWith<K>>::Archived,
            <B as ArchiveWith<V>>::Archived,
        >,
        deserializer: &mut D,
    ) -> Result<HashMap<K, V, S>, <D as Fallible>::Error> {
        let mut result =
            HashMap::with_capacity_and_hasher(field.len(), S::default());
        for (k, v) in field.iter() {
            result.insert(
                A::deserialize_with(k, deserializer)?,
                B::deserialize_with(v, deserializer)?,
            );
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use core::hash::BuildHasherDefault;

    use rkyv_derive::{Archive, Deserialize, Serialize};

    use super::HashMap;
    use crate::{
        api::test::to_archived,
        hash::FxHasher64,
        with::{InlineAsBox, MapKV},
    };

    #[test]
    fn with_as_mapkv() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[rkyv(with = MapKV<InlineAsBox, InlineAsBox>)]
            a: HashMap<&'a str, &'a str, BuildHasherDefault<FxHasher64>>,
        }

        let mut a =
            HashMap::with_hasher(BuildHasherDefault::<FxHasher64>::default());
        a.insert("foo", "bar");
        a.insert("woo", "roo");

        let value = Test { a };

        to_archived(&value, |archived| {
            assert_eq!(archived.a.len(), 2);
            assert!(archived.a.contains_key("foo"));
            assert_eq!(**archived.a.get("woo").unwrap(), *"roo");
        });
    }
}
