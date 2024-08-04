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

macro_rules! impl_serialize_with_as_atomic {
    ($atomic:ty) => {
        impl<S, SO, DO> $crate::with::SerializeWith<$atomic, S>
            for $crate::with::AsAtomic<SO, DO>
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
