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

# Test matrices

For each matrix, select one feature from each group to enable and run a check
build.

## Primitives

- `little_endian`, `big_endian`
- `aligned`, `unaligned`
- `pointer_width_16`, `pointer_width_32`, `pointer_width_64`

Builds:

```sh
cargo test --tests --no-default-features --features "little_endian pointer_width_16" >> results.txt~
cargo test --tests --no-default-features --features "big_endian pointer_width_16" >> results.txt~
cargo test --tests --no-default-features --features "little_endian unaligned pointer_width_16" >> results.txt~
cargo test --tests --no-default-features --features "big_endian unaligned pointer_width_16" >> results.txt~
cargo test --tests --no-default-features --features "little_endian pointer_width_32" >> results.txt~
cargo test --tests --no-default-features --features "big_endian pointer_width_32" >> results.txt~
cargo test --tests --no-default-features --features "little_endian unaligned pointer_width_32" >> results.txt~
cargo test --tests --no-default-features --features "big_endian unaligned pointer_width_32" >> results.txt~
cargo test --tests --no-default-features --features "little_endian pointer_width_64" >> results.txt~
cargo test --tests --no-default-features --features "big_endian pointer_width_64" >> results.txt~
cargo test --tests --no-default-features --features "little_endian unaligned pointer_width_64" >> results.txt~
cargo test --tests --no-default-features --features "big_endian unaligned pointer_width_64" >> results.txt~
```

## Features

- none, `alloc`, `std`
- none, `bytecheck`
- none, all external crates
  - `hashbrown-0_14`
  - `indexmap-2`
  - `smallvec-1`
  - `smol_str-0_2`
  - `smol_str-0_3`
  - `arrayvec-0_7`
  - `tinyvec-1`
  - `uuid-1`
  - `bytes-1`
  - `thin-vec-0_2`
  - `triomphe-0_1`

Builds:

```sh
cargo test --tests --no-default-features >> results.txt~
cargo test --tests --no-default-features --features "alloc" >> results.txt~
cargo test --tests --no-default-features --features "std" >> results.txt~
cargo test --tests --no-default-features --features "bytecheck" >> results.txt~
cargo test --tests --no-default-features --features "bytecheck alloc" >> results.txt~
cargo test --tests --no-default-features --features "bytecheck std" >> results.txt~
cargo test --tests --no-default-features --features "hashbrown-0_14 indexmap-2 smallvec-1 smol_str-0_2 smol_str-0_3 arrayvec-0_7 tinyvec-1 uuid-1 bytes-1 thin-vec-0_2 triomphe-0_1" >> results.txt~
cargo test --tests --no-default-features --features "alloc hashbrown-0_14 indexmap-2 smallvec-1 smol_str-0_2 smol_str-0_3 arrayvec-0_7 tinyvec-1 uuid-1 bytes-1 thin-vec-0_2 triomphe-0_1" >> results.txt~
cargo test --tests --no-default-features --features "std hashbrown-0_14 indexmap-2 smallvec-1 smol_str-0_2 smol_str-0_3 arrayvec-0_7 tinyvec-1 uuid-1 bytes-1 thin-vec-0_2 triomphe-0_1" >> results.txt~
cargo test --tests --no-default-features --features "bytecheck hashbrown-0_14 indexmap-2 smallvec-1 smol_str-0_2 smol_str-0_3 arrayvec-0_7 tinyvec-1 uuid-1 bytes-1 thin-vec-0_2 triomphe-0_1" >> results.txt~
cargo test --tests --no-default-features --features "bytecheck alloc hashbrown-0_14 indexmap-2 smallvec-1 smol_str-0_2 smol_str-0_3 arrayvec-0_7 tinyvec-1 uuid-1 bytes-1 thin-vec-0_2 triomphe-0_1" >> results.txt~
cargo test --tests --no-default-features --features "bytecheck std hashbrown-0_14 indexmap-2 smallvec-1 smol_str-0_2 smol_str-0_3 arrayvec-0_7 tinyvec-1 uuid-1 bytes-1 thin-vec-0_2 triomphe-0_1" >> results.txt~
```

# Testing through MIRI

MIRI's default aliasing model is stacked borrows, which doesn't support relative
pointers even though Rust's memory model does. The experimental tree borrows
aliasing model supports relative pointers, so we use that instead:

```sh
$env:MIRIFLAGS="-Zmiri-tree-borrows"
```
