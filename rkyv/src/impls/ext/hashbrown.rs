macro_rules! impl_hashbrown {
    ($hashbrown:ident) => {
        mod hash_map {
            use core::{
                borrow::Borrow,
                hash::{BuildHasher, Hash},
            };

            use $hashbrown::HashMap;
            use rancor::{Fallible, Source};

            use crate::{
                collections::swiss_table::map::{
                    ArchivedHashMap, HashMapResolver,
                },
                ser::{Allocator, Writer},
                Archive, Deserialize, Place, Serialize,
            };

            impl<K, V: Archive, S> Archive for HashMap<K, V, S>
            where
                K: Archive + Hash + Eq,
                K::Archived: Hash + Eq,
            {
                type Archived = ArchivedHashMap<K::Archived, V::Archived>;
                type Resolver = HashMapResolver;

                fn resolve(
                    &self,
                    resolver: Self::Resolver,
                    out: Place<Self::Archived>,
                ) {
                    ArchivedHashMap::resolve_from_len(
                        self.len(),
                        (7, 8),
                        resolver,
                        out,
                    );
                }
            }

            impl<K, V, S, RandomState> Serialize<S>
                for HashMap<K, V, RandomState>
            where
                K: Serialize<S> + Hash + Eq,
                K::Archived: Hash + Eq,
                V: Serialize<S>,
                S: Fallible + Writer + Allocator + ?Sized,
                S::Error: Source,
            {
                fn serialize(
                    &self,
                    serializer: &mut S,
                ) -> Result<Self::Resolver, S::Error> {
                    ArchivedHashMap::<K::Archived, V::Archived>::
                        serialize_from_iter::<_, _, _, K, V, _>(
                            self.iter(),
                            (7, 8),
                            serializer,
                        )
                }
            }

            impl<K, V, D, S> Deserialize<HashMap<K, V, S>, D>
                for ArchivedHashMap<K::Archived, V::Archived>
            where
                K: Archive + Hash + Eq,
                K::Archived: Deserialize<K, D> + Hash + Eq,
                V: Archive,
                V::Archived: Deserialize<V, D>,
                D: Fallible + ?Sized,
                S: Default + BuildHasher,
            {
                fn deserialize(
                    &self,
                    deserializer: &mut D,
                ) -> Result<HashMap<K, V, S>, D::Error> {
                    let mut result = HashMap::with_capacity_and_hasher(
                        self.len(),
                        S::default(),
                    );
                    for (k, v) in self.iter() {
                        result.insert(
                            k.deserialize(deserializer)?,
                            v.deserialize(deserializer)?,
                        );
                    }
                    Ok(result)
                }
            }

            impl<K, V, AK, AV, S> PartialEq<HashMap<K, V, S>>
                for ArchivedHashMap<AK, AV>
            where
                K: Hash + Eq + Borrow<AK>,
                AK: Hash + Eq,
                AV: PartialEq<V>,
                S: BuildHasher,
            {
                fn eq(&self, other: &HashMap<K, V, S>) -> bool {
                    if self.len() != other.len() {
                        false
                    } else {
                        self.iter().all(|(key, value)| {
                            other.get(key).map_or(false, |v| value.eq(v))
                        })
                    }
                }
            }

            #[cfg(test)]
            mod tests {
                use core::hash::BuildHasherDefault;

                use super::HashMap;
                use crate::{
                    alloc::string::String,
                    api::test::roundtrip_with,
                    hash::FxHasher64,
                };

                #[test]
                fn index_map() {
                    let mut value = HashMap::with_hasher(
                        BuildHasherDefault::<FxHasher64>::default(),
                    );
                    value.insert(String::from("foo"), 10);
                    value.insert(String::from("bar"), 20);
                    value.insert(String::from("baz"), 40);
                    value.insert(String::from("bat"), 80);

                    roundtrip_with(&value, |a, b| {
                        assert_eq!(a.len(), b.len());
                        for (k, v) in a.iter() {
                            let (ak, av) = b.get_key_value(k.as_str()).unwrap();
                            assert_eq!(k, ak);
                            assert_eq!(v, av);
                        }
                    });
                }
            }
        }

        mod hash_set {
            use core::{
                borrow::Borrow,
                hash::{BuildHasher, Hash},
            };

            use $hashbrown::HashSet;
            use rancor::{Fallible, Source};

            use crate::{
                collections::swiss_table::set::{
                    ArchivedHashSet, HashSetResolver,
                },
                ser::{Allocator, Writer},
                Archive, Deserialize, Place, Serialize,
            };

            impl<K, S> Archive for HashSet<K, S>
            where
                K: Archive + Hash + Eq,
                K::Archived: Hash + Eq,
            {
                type Archived = ArchivedHashSet<K::Archived>;
                type Resolver = HashSetResolver;

                fn resolve(
                    &self,
                    resolver: Self::Resolver,
                    out: Place<Self::Archived>,
                ) {
                    ArchivedHashSet::<K::Archived>::resolve_from_len(
                        self.len(),
                        (7, 8),
                        resolver,
                        out,
                    );
                }
            }

            impl<K, S, RS> Serialize<S> for HashSet<K, RS>
            where
                K::Archived: Hash + Eq,
                K: Serialize<S> + Hash + Eq,
                S: Fallible + Allocator + Writer + ?Sized,
                S::Error: Source,
            {
                fn serialize(
                    &self,
                    serializer: &mut S,
                ) -> Result<Self::Resolver, S::Error> {
                    ArchivedHashSet::<K::Archived>::serialize_from_iter::<
                        _,
                        K,
                        _,
                    >(self.iter(), (7, 8), serializer)
                }
            }

            impl<K, D, S> Deserialize<HashSet<K, S>, D>
                for ArchivedHashSet<K::Archived>
            where
                K: Archive + Hash + Eq,
                K::Archived: Deserialize<K, D> + Hash + Eq,
                D: Fallible + ?Sized,
                S: Default + BuildHasher,
            {
                fn deserialize(
                    &self,
                    deserializer: &mut D,
                ) -> Result<HashSet<K, S>, D::Error> {
                    let mut result = HashSet::with_hasher(S::default());
                    for k in self.iter() {
                        result.insert(k.deserialize(deserializer)?);
                    }
                    Ok(result)
                }
            }

            impl<K: Hash + Eq + Borrow<AK>, AK: Hash + Eq, S: BuildHasher>
                PartialEq<HashSet<K, S>> for ArchivedHashSet<AK>
            {
                fn eq(&self, other: &HashSet<K, S>) -> bool {
                    if self.len() != other.len() {
                        false
                    } else {
                        self.iter().all(|key| other.get(key).is_some())
                    }
                }
            }

            impl<K: Hash + Eq + Borrow<AK>, AK: Hash + Eq, S: BuildHasher>
                PartialEq<ArchivedHashSet<AK>> for HashSet<K, S>
            {
                fn eq(&self, other: &ArchivedHashSet<AK>) -> bool {
                    other.eq(self)
                }
            }

            #[cfg(test)]
            mod tests {
                use core::hash::BuildHasherDefault;

                use super::HashSet;
                use crate::{
                    alloc::string::String,
                    api::test::roundtrip_with,
                    hash::FxHasher64,
                };

                #[test]
                fn index_set() {
                    let mut value = HashSet::with_hasher(
                        BuildHasherDefault::<FxHasher64>::default(),
                    );
                    value.insert(String::from("foo"));
                    value.insert(String::from("bar"));
                    value.insert(String::from("baz"));
                    value.insert(String::from("bat"));

                    roundtrip_with(&value, |a, b| {
                        assert_eq!(a.len(), b.len());
                        for k in a.iter() {
                            let ak = b.get(k.as_str()).unwrap();
                            assert_eq!(k, ak);
                        }
                    });
                }
            }
        }

        mod with {
            use core::{
                hash::{BuildHasher, Hash},
                marker::PhantomData,
            };

            use $hashbrown::HashMap;
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
                    ArchivedHashMap::resolve_from_len(
                        field.len(),
                        (7, 8),
                        resolver,
                        out,
                    )
                }
            }

            impl<A, B, K, V, H, S> SerializeWith<HashMap<K, V, H>, S>
                for MapKV<A, B>
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
                A: ArchiveWith<K>
                    + DeserializeWith<<A as ArchiveWith<K>>::Archived, K, D>,
                B: ArchiveWith<V>
                    + DeserializeWith<<B as ArchiveWith<V>>::Archived, V, D>,
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
                    let mut result = HashMap::with_capacity_and_hasher(
                        field.len(),
                        S::default(),
                    );
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
                        a: HashMap<
                            &'a str,
                            &'a str,
                            BuildHasherDefault<FxHasher64>,
                        >,
                    }

                    let mut a = HashMap::with_hasher(
                        BuildHasherDefault::<FxHasher64>::default(),
                    );
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
        }
    };
}

#[cfg(feature = "hashbrown-0_14")]
mod hashbrown_0_14 {
    impl_hashbrown!(hashbrown_0_14);
}

#[cfg(feature = "hashbrown-0_15")]
mod hashbrown_0_15 {
    impl_hashbrown!(hashbrown_0_15);
}

#[cfg(feature = "hashbrown-0_16")]
mod hashbrown_0_16 {
    impl_hashbrown!(hashbrown);
}
