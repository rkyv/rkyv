use std::{cell::Cell, collections::VecDeque, sync::Arc};

use rancor::{Fallible, Source};

use crate::{Archive, Deserialize, Place, Serialize};

std::thread_local! {
    static DROPS: Cell<usize> = const { Cell::new(0) };
}

struct Refusing(u8);

impl Drop for Refusing {
    fn drop(&mut self) {
        DROPS.with(|drops| drops.set(drops.get() + 1));
    }
}

impl Archive for Refusing {
    type Archived = <u8 as Archive>::Archived;
    type Resolver = <u8 as Archive>::Resolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        self.0.resolve(resolver, out);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for Refusing {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        self.0.serialize(serializer)
    }
}

#[derive(Debug)]
struct RefusedElement;

impl core::fmt::Display for RefusedElement {
    fn fmt(
        &self,
        formatter: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        formatter.write_str("refused element")
    }
}

impl core::error::Error for RefusedElement {}

impl<D> Deserialize<Refusing, D> for u8
where
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize(&self, _: &mut D) -> Result<Refusing, D::Error> {
        if *self == 2 {
            return Err(D::Error::new(RefusedElement));
        }
        Ok(Refusing(*self))
    }
}

#[derive(Archive, Serialize, Deserialize)]
#[rkyv(crate)]
struct Vector {
    values: Vec<Refusing>,
}

#[derive(Archive, Serialize, Deserialize)]
#[rkyv(crate)]
struct Deque {
    values: VecDeque<Refusing>,
}

#[derive(Archive, Serialize, Deserialize)]
#[rkyv(crate)]
struct Boxed {
    values: Box<[Refusing]>,
}

#[derive(Archive, Serialize, Deserialize)]
#[rkyv(crate)]
struct Shared {
    values: Arc<[Refusing]>,
}

#[derive(Archive, Serialize, Deserialize)]
#[rkyv(crate)]
struct FixedArray {
    values: [Refusing; 2],
}

macro_rules! assert_drops_initialized_prefix {
    ($value:expr, $type:ty) => {{
        let value = $value;
        let bytes = crate::to_bytes::<rancor::Error>(&value).unwrap();
        drop(value);
        DROPS.with(|drops| drops.set(0));
        assert!(crate::from_bytes::<$type, rancor::Error>(&bytes).is_err());
        DROPS.with(|drops| assert_eq!(drops.get(), 1));
    }};
}

#[test]
fn refused_vector_drops_its_initialized_prefix() {
    assert_drops_initialized_prefix!(
        Vector {
            values: vec![Refusing(1), Refusing(2)],
        },
        Vector
    );
}

#[test]
fn refused_deque_drops_its_initialized_prefix() {
    assert_drops_initialized_prefix!(
        Deque {
            values: VecDeque::from(vec![Refusing(1), Refusing(2)]),
        },
        Deque
    );
}

#[test]
fn refused_boxed_slice_drops_its_initialized_prefix() {
    assert_drops_initialized_prefix!(
        Boxed {
            values: vec![Refusing(1), Refusing(2)].into_boxed_slice(),
        },
        Boxed
    );
}

#[test]
fn refused_shared_slice_drops_its_initialized_prefix() {
    assert_drops_initialized_prefix!(
        Shared {
            values: Arc::from(
                vec![Refusing(1), Refusing(2)].into_boxed_slice()
            ),
        },
        Shared
    );
}

#[test]
fn refused_fixed_array_drops_its_initialized_prefix() {
    assert_drops_initialized_prefix!(
        FixedArray {
            values: [Refusing(1), Refusing(2)],
        },
        FixedArray
    );
}
