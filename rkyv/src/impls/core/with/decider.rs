use crate::{
    niche::decider::{Decider, NaN},
    Archive, Archived, Place, Resolver,
};

macro_rules! impl_float_nan_decider {
    ($fl:ty) => {
        impl Decider<$fl> for NaN {
            type Archived = Archived<$fl>;

            fn as_option(archived: &Self::Archived) -> Option<&Archived<$fl>> {
                if archived.to_native().is_nan() {
                    None
                } else {
                    Some(archived)
                }
            }

            fn resolve_from_option(
                option: Option<&$fl>,
                resolver: Option<Resolver<$fl>>,
                out: Place<Self::Archived>,
            ) {
                match option {
                    Some(value) => {
                        let resolver = resolver.expect("non-niched resolver");
                        value.resolve(resolver, out);
                    }
                    None => <$fl>::resolve(&<$fl>::NAN, (), out),
                }
            }
        }
    };
}

impl_float_nan_decider!(f32);
impl_float_nan_decider!(f64);
