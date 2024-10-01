use core::mem::MaybeUninit;

use crate::Rng;

pub trait Generate {
    fn generate<R: Rng>(rng: &mut R) -> Self;
}

impl Generate for () {
    fn generate<R: Rng>(_: &mut R) -> Self {}
}

impl Generate for bool {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        rng.gen_bool(0.5)
    }
}

impl Generate for u32 {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        rng.gen()
    }
}

impl Generate for f32 {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        rng.gen()
    }
}

impl Generate for f64 {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        rng.gen()
    }
}

impl<T: Generate, const N: usize> Generate for [T; N] {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        let mut result = MaybeUninit::<[T; N]>::uninit();
        for i in 0..N {
            unsafe {
                result
                    .as_mut_ptr()
                    .cast::<T>()
                    .add(i)
                    .write(T::generate(rng));
            }
        }
        unsafe { result.assume_init() }
    }
}

impl<T: Generate> Generate for Option<T> {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        if rng.gen_bool(0.5) {
            Some(T::generate(rng))
        } else {
            None
        }
    }
}

pub fn generate_vec<R: Rng, T: Generate>(
    rng: &mut R,
    range: core::ops::Range<usize>,
) -> Vec<T> {
    let len = rng.gen_range(range);
    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        result.push(T::generate(rng));
    }
    result
}
