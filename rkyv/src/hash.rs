use core::hash::Hash;

pub fn hash_value<Q>(value: &Q) -> u64
where
    Q: Hash + ?Sized,
{
    use core::hash::Hasher;

    use seahash::SeaHasher;

    // TODO: switch hasher / pick nothing-up-my-sleeve numbers for initial
    // state seeds
    let mut state = SeaHasher::with_seeds(
        0x00000000_00000000,
        0x00000000_00000000,
        0x00000000_00000000,
        0x00000000_00000000,
    );
    value.hash(&mut state);
    state.finish()
}
