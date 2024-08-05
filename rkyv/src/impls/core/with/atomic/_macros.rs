macro_rules! impl_serialize_with_atomic_load {
    ($atomic:ty) => {
        impl<S, SO> $crate::with::SerializeWith<$atomic, S>
            for $crate::with::AtomicLoad<SO>
        where
            S: $crate::rancor::Fallible + ?Sized,
            SO: $crate::impls::core::with::atomic::LoadOrdering,
        {
            fn serialize_with(
                _: &$atomic,
                _: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }
    };
}
