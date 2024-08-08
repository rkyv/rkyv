




impl<A: ArchiveWith<K>, B: ArchiveWith<V>, K, V> ArchiveWith<hashbrown::HashMap<K, V>> for MapKV<A, B>
{
    type Archived = ArchivedHashMap<<A as ArchiveWith<K>>::Archived, <B as ArchiveWith<V>>::Archived>;
    type Resolver = HashMapResolver;

    fn resolve_with(
            field: &hashbrown::HashMap<K, V>,
            resolver: Self::Resolver,
            out: Place<Self::Archived>,
        ) {
        ArchivedHashMap::resolve_from_len(field.len(), (7, 8), resolver, out)
    }
}

impl<A, B, K, V, S> SerializeWith<hashbrown::HashMap<K, V>, S> for MapKV<A, B>
where
    A: ArchiveWith<K> + SerializeWith<K, S>,
    B: ArchiveWith<V> + SerializeWith<V, S>,
    K: Hash + Eq,
    <A as ArchiveWith<K>>::Archived: Eq + Hash,
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source
{

    fn serialize_with(
            field: &hashbrown::HashMap<K, V>,
            serializer: &mut S,
        ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedHashMap::<_, _, FxHasher64>::serialize_from_iter(field.iter()
            .map(|(k, v)| {
                (RefWrapper::<'_, A, K>(k, PhantomData::<A>), RefWrapper::<'_, B, V>(v, PhantomData::<B>))
            })
            , (7, 8), serializer)
    } 
}

impl<A, B, K, V, D, S> DeserializeWith<ArchivedHashMap<<A as ArchiveWith<K>>::Archived, <B as ArchiveWith<V>>::Archived>, hashbrown::HashMap<K, V, S>, D> for MapKV<A, B>
where
    A: ArchiveWith<K> + DeserializeWith<<A as ArchiveWith<K>>::Archived, K, D>,
    B: ArchiveWith<V> + DeserializeWith<<B as ArchiveWith<V>>::Archived, V, D>,
    K: Ord + Hash + Eq,
    D: Fallible + ?Sized,
    S: Default + BuildHasher
{

    fn deserialize_with(field: &ArchivedHashMap<<A as ArchiveWith<K>>::Archived, <B as ArchiveWith<V>>::Archived>, deserializer: &mut D)
            -> Result<hashbrown::HashMap<K, V, S>, <D as Fallible>::Error> {
        let mut result = hashbrown::HashMap::with_capacity_and_hasher(field.len(), S::default());
        for (k, v) in field.iter() {
            result.insert(
                A::deserialize_with(k, deserializer)?,
                B::deserialize_with(v, deserializer)?
            );
        }
        Ok(result)

    }

}


