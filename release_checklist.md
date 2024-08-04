# Release checklist

- [ ] Run `cargo clippy` and `cargo +nightly fmt` on all crates
- [ ] Generate documentation with `cargo +nightly doc --open` and make sure every public item is documented
- [ ] Make sure that `crates-io.md` and `README.md` are up to date with the most recent examples
- [ ] Build using all supported feature combinations
- [ ] Test using all supported feature combinations
- [ ] Bump version numbers and check that all crates have their dependencies updated to match
- [ ] Commit with the name `"Release X.X.X"` and push
- [ ] Merge development branch into `master`
- [ ] Wait for CI tests to pass
- [ ] Publish crates
- [ ] Create release and tag version `"vX.X.X"` with a description that directly links to the issues related to the release
- [ ] Close milestone
- [ ] If sufficiently major, post on social media:
  - [ ] `/r/rust_gamedev`
  - [ ] `/r/rust`
  - [ ] Twitter

# Check matrices

For each matrix, select one feature from each group to enable and run a check
build.

## Primitives

- `little_endian`, `big_endian`
- none, `unaligned`
- `pointer_width_16`, `pointer_width_32`, `pointer_width_64`

Builds:

```sh
cargo test --tests --no-default-features --features "little_endian pointer_width_16" >> results.txt
cargo test --tests --no-default-features --features "big_endian pointer_width_16" >> results.txt
cargo test --tests --no-default-features --features "little_endian unaligned pointer_width_16" >> results.txt
cargo test --tests --no-default-features --features "big_endian unaligned pointer_width_16" >> results.txt
cargo test --tests --no-default-features --features "little_endian pointer_width_32" >> results.txt
cargo test --tests --no-default-features --features "big_endian pointer_width_32" >> results.txt
cargo test --tests --no-default-features --features "little_endian unaligned pointer_width_32" >> results.txt
cargo test --tests --no-default-features --features "big_endian unaligned pointer_width_32" >> results.txt
cargo test --tests --no-default-features --features "little_endian pointer_width_64" >> results.txt
cargo test --tests --no-default-features --features "big_endian pointer_width_64" >> results.txt
cargo test --tests --no-default-features --features "little_endian unaligned pointer_width_64" >> results.txt
cargo test --tests --no-default-features --features "big_endian unaligned pointer_width_64" >> results.txt
```

## Features

- none, `alloc`, `std`
- none, `bytecheck`
- none, all external crates
  - `bitvec`
  - `hashbrown`
  - `indexmap`
  - `smallvec`
  - `smol_str`
  - `arrayvec`
  - `tinyvec`
  - `uuid`
  - `bytes`
  - `thin-vec`
  - `triomphe`

Builds:

```sh
cargo test --tests --no-default-features
cargo test --tests --no-default-features --features "alloc"
cargo test --tests --no-default-features --features "std"
cargo test --tests --no-default-features --features "bytecheck"
cargo test --tests --no-default-features --features "bytecheck alloc"
cargo test --tests --no-default-features --features "bytecheck std"
cargo test --tests --no-default-features --features "bitvec hashbrown indexmap smallvec smol_str arrayvec tinyvec uuid bytes thin-vec triomphe"
cargo test --tests --no-default-features --features "alloc bitvec hashbrown indexmap smallvec smol_str arrayvec tinyvec uuid bytes thin-vec triomphe"
cargo test --tests --no-default-features --features "std bitvec hashbrown indexmap smallvec smol_str arrayvec tinyvec uuid bytes thin-vec triomphe"
cargo test --tests --no-default-features --features "bytecheck bitvec hashbrown indexmap smallvec smol_str arrayvec tinyvec uuid bytes thin-vec triomphe"
cargo test --tests --no-default-features --features "bytecheck alloc bitvec hashbrown indexmap smallvec smol_str arrayvec tinyvec uuid bytes thin-vec triomphe"
cargo test --tests --no-default-features --features "bytecheck std bitvec hashbrown indexmap smallvec smol_str arrayvec tinyvec uuid bytes thin-vec triomphe"
```

# 

# Testing through MIRI

MIRI's default aliasing model is stacked borrows, which doesn't support relative
pointers even though Rust's memory model does. The experimental tree borrows
aliasing model supports relative pointers, so we use that instead:

```sh
$env:MIRIFLAGS="-Zmiri-disable-stacked-borrows -Zmiri-tree-borrows"
```
