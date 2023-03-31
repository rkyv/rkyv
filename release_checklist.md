# Release checklist

- [ ] Run `cargo clippy` and `cargo fmt` on all crates
- [ ] Generate documentation with `cargo doc --open` and make sure:
  - [ ] Every public item is documented
  - [ ] Every link is correct
- [ ] Make sure that `crates-io.md` and `README.md` are up to date with the most recent examples
- [ ] Run all tests with all combinations of features in debug and release
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

# All tests cheatsheet

- Regular tests:
  - [ ] `cargo test --no-default-features --features "size_32"`
  - [ ] `cargo test --no-default-features --features "size_32 alloc"`
  - [ ] `cargo test --no-default-features --features "size_32 validation"`
  - [ ] `cargo test --no-default-features --features "size_32 std"`
  - [ ] `cargo test --no-default-features --features "size_32 std validation"`
  - [ ] `cargo test --no-default-features --features "size_32 std validation strict"`
  - [ ] `cargo test --no-default-features --features "size_16 archive_le std validation"`
  - [ ] `cargo test --no-default-features --features "size_32 archive_le std validation"`
  - [ ] `cargo test --no-default-features --features "size_64 archive_le std validation"`
  - [ ] `cargo test --no-default-features --features "size_16 archive_be std validation"`
  - [ ] `cargo test --no-default-features --features "size_32 archive_be std validation"`
  - [ ] `cargo test --no-default-features --features "size_64 archive_be std validation"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy alloc"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy alloc validation"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy std"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy std validation"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy std validation strict"`
- Release tests:
  - [ ] `cargo test --release --no-default-features --features "size_32"`
  - [ ] `cargo test --release --no-default-features --features "size_32 alloc"`
  - [ ] `cargo test --release --no-default-features --features "size_32 validation"`
  - [ ] `cargo test --release --no-default-features --features "size_32 std"`
  - [ ] `cargo test --release --no-default-features --features "size_32 std validation"`
  - [ ] `cargo test --release --no-default-features --features "size_32 std validation strict"`
  - [ ] `cargo test --release --no-default-features --features "size_16 archive_le std validation"`
  - [ ] `cargo test --release --no-default-features --features "size_32 archive_le std validation"`
  - [ ] `cargo test --release --no-default-features --features "size_64 archive_le std validation"`
  - [ ] `cargo test --release --no-default-features --features "size_16 archive_be std validation"`
  - [ ] `cargo test --release --no-default-features --features "size_32 archive_be std validation"`
  - [ ] `cargo test --release --no-default-features --features "size_64 archive_be std validation"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy alloc"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy alloc validation"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy std"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy std validation"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy std validation strict"`
- Miri tests:
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 alloc"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 std"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 std validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 std validation strict"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_16 archive_le std validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 archive_le std validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_64 archive_le std validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_16 archive_be std validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 archive_be std validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_64 archive_be std validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy alloc"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy alloc validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy std"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy std validation"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy std validation strict"`
- Wasm-pack tests:
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 alloc"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 validation"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 std"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 std validation"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 std validation strict"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_16 archive_le std validation"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 archive_le std validation"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_16 archive_be std validation"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 archive_be std validation"`

# Copy-paste version

Remember to use tree borrows instead of stacked borrows:

```sh
$env:MIRIFLAGS="-Zmiri-disable-stacked-borrows -Zmiri-tree-borrows"
```

```sh
cargo test --no-default-features --features "size_32" >> results.txt
cargo test --no-default-features --features "size_32 alloc" >> results.txt
cargo test --no-default-features --features "size_32 validation" >> results.txt
cargo test --no-default-features --features "size_32 std" >> results.txt
cargo test --no-default-features --features "size_32 std validation" >> results.txt
cargo test --no-default-features --features "size_32 std validation strict" >> results.txt
cargo test --no-default-features --features "size_16 archive_le std validation" >> results.txt
cargo test --no-default-features --features "size_32 archive_le std validation" >> results.txt
cargo test --no-default-features --features "size_64 archive_le std validation" >> results.txt
cargo test --no-default-features --features "size_16 archive_be std validation" >> results.txt
cargo test --no-default-features --features "size_32 archive_be std validation" >> results.txt
cargo test --no-default-features --features "size_64 archive_be std validation" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy alloc" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy alloc validation" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy std" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy std validation" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy std validation strict" >> results.txt
cargo test --release --no-default-features --features "size_32" >> results.txt
cargo test --release --no-default-features --features "size_32 alloc" >> results.txt
cargo test --release --no-default-features --features "size_32 validation" >> results.txt
cargo test --release --no-default-features --features "size_32 std" >> results.txt
cargo test --release --no-default-features --features "size_32 std validation" >> results.txt
cargo test --release --no-default-features --features "size_32 std validation strict" >> results.txt
cargo test --release --no-default-features --features "size_16 archive_le std validation" >> results.txt
cargo test --release --no-default-features --features "size_32 archive_le std validation" >> results.txt
cargo test --release --no-default-features --features "size_64 archive_le std validation" >> results.txt
cargo test --release --no-default-features --features "size_16 archive_be std validation" >> results.txt
cargo test --release --no-default-features --features "size_32 archive_be std validation" >> results.txt
cargo test --release --no-default-features --features "size_64 archive_be std validation" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy alloc" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy alloc validation" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy std" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy std validation" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy std validation strict" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 alloc" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 std" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 std validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 std validation strict" >> results.txt
cargo +nightly miri test --no-default-features --features "size_16 archive_le std validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 archive_le std validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_64 archive_le std validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_16 archive_be std validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 archive_be std validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_64 archive_be std validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy alloc" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy alloc validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy std" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy std validation" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy std validation strict" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 alloc" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 validation" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 std" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 std validation" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 std validation strict" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_16 archive_le std validation" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 archive_le std validation" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_16 archive_be std validation" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 archive_be std validation" >> results.txt
```
