mod generate;

pub use divan;
pub use rand::Rng;
use rand_pcg::Lcg64Xsh32;

pub use self::generate::*;

#[macro_export]
macro_rules! bench_dataset {
    ($ty:ty = $generate:expr) => {
        #[$crate::divan::bench(min_time = std::time::Duration::from_secs(3))]
        pub fn ser(bencher: $crate::divan::Bencher) {
            let data = $generate;
            let mut bytes = rkyv::util::AlignedVec::<16>::new();

            bencher.bench_local(|| {
                let mut buffer = core::mem::take(&mut bytes);
                buffer.clear();

                bytes = $crate::divan::black_box(
                    rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Failure>(
                        $crate::divan::black_box(&data),
                        $crate::divan::black_box(buffer),
                    )
                    .unwrap(),
                );
            });
        }

        #[$crate::divan::bench(min_time = std::time::Duration::from_secs(3))]
        pub fn de(bencher: $crate::divan::Bencher) {
            let bytes =
                rkyv::api::high::to_bytes::<rkyv::rancor::Failure>(&$generate)
                    .unwrap();

            bencher.bench_local(|| {
                rkyv::from_bytes::<$ty, rkyv::rancor::Failure>(
                    $crate::divan::black_box(&bytes),
                )
                .unwrap()
            })
        }

        #[$crate::divan::bench(min_time = std::time::Duration::from_secs(3))]
        pub fn check(bencher: $crate::divan::Bencher) {
            let bytes =
                rkyv::api::high::to_bytes::<rkyv::rancor::Failure>(&$generate)
                    .unwrap();

            bencher.bench_local(|| {
                rkyv::access::<rkyv::Archived<$ty>, rkyv::rancor::Failure>(
                    $crate::divan::black_box(&bytes),
                )
            })
        }

        fn main() {
            $crate::divan::main();
        }
    };
}

pub fn rng() -> Lcg64Xsh32 {
    // nothing up our sleeves, state and stream are first 20 digits of pi
    const STATE: u64 = 3141592653;
    const STREAM: u64 = 5897932384;

    Lcg64Xsh32::new(STATE, STREAM)
}
