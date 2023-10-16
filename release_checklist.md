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
  - [ ] `cargo test --no-default-features --features "size_32 bytecheck"`
  - [ ] `cargo test --no-default-features --features "size_32 std"`
  - [ ] `cargo test --no-default-features --features "size_32 std bytecheck"`
  - [ ] `cargo test --no-default-features --features "size_32 std bytecheck strict"`
  - [ ] `cargo test --no-default-features --features "size_16 archive_le std bytecheck"`
  - [ ] `cargo test --no-default-features --features "size_32 archive_le std bytecheck"`
  - [ ] `cargo test --no-default-features --features "size_64 archive_le std bytecheck"`
  - [ ] `cargo test --no-default-features --features "size_16 archive_be std bytecheck"`
  - [ ] `cargo test --no-default-features --features "size_32 archive_be std bytecheck"`
  - [ ] `cargo test --no-default-features --features "size_64 archive_be std bytecheck"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy alloc"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy alloc bytecheck"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy std"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy std bytecheck"`
  - [ ] `cargo +nightly test --no-default-features --features "size_32 copy std bytecheck strict"`
- Release tests:
  - [ ] `cargo test --release --no-default-features --features "size_32"`
  - [ ] `cargo test --release --no-default-features --features "size_32 alloc"`
  - [ ] `cargo test --release --no-default-features --features "size_32 bytecheck"`
  - [ ] `cargo test --release --no-default-features --features "size_32 std"`
  - [ ] `cargo test --release --no-default-features --features "size_32 std bytecheck"`
  - [ ] `cargo test --release --no-default-features --features "size_32 std bytecheck strict"`
  - [ ] `cargo test --release --no-default-features --features "size_16 archive_le std bytecheck"`
  - [ ] `cargo test --release --no-default-features --features "size_32 archive_le std bytecheck"`
  - [ ] `cargo test --release --no-default-features --features "size_64 archive_le std bytecheck"`
  - [ ] `cargo test --release --no-default-features --features "size_16 archive_be std bytecheck"`
  - [ ] `cargo test --release --no-default-features --features "size_32 archive_be std bytecheck"`
  - [ ] `cargo test --release --no-default-features --features "size_64 archive_be std bytecheck"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy alloc"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy alloc bytecheck"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy std"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy std bytecheck"`
  - [ ] `cargo +nightly test --release --no-default-features --features "size_32 copy std bytecheck strict"`
- Miri tests:
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 alloc"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 std"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 std bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 std bytecheck strict"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_16 archive_le std bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 archive_le std bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_64 archive_le std bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_16 archive_be std bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 archive_be std bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_64 archive_be std bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy alloc"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy alloc bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy std"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy std bytecheck"`
  - [ ] `cargo +nightly miri test --no-default-features --features "size_32 copy std bytecheck strict"`
- Wasm-pack tests:
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 alloc"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 bytecheck"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 std"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 std bytecheck"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 std bytecheck strict"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_16 archive_le std bytecheck"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 archive_le std bytecheck"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_16 archive_be std bytecheck"`
  - [ ] `wasm-pack test --node -- --no-default-features --features "wasm size_32 archive_be std bytecheck"`

# Copy-paste version

Remember to use tree borrows instead of stacked borrows:

```sh
$env:MIRIFLAGS="-Zmiri-disable-stacked-borrows -Zmiri-tree-borrows"
```

```sh
cargo test --no-default-features --features "size_32" >> results.txt
cargo test --no-default-features --features "size_32 alloc" >> results.txt
cargo test --no-default-features --features "size_32 bytecheck" >> results.txt
cargo test --no-default-features --features "size_32 std" >> results.txt
cargo test --no-default-features --features "size_32 std bytecheck" >> results.txt
cargo test --no-default-features --features "size_32 std bytecheck strict" >> results.txt
cargo test --no-default-features --features "size_16 archive_le std bytecheck" >> results.txt
cargo test --no-default-features --features "size_32 archive_le std bytecheck" >> results.txt
cargo test --no-default-features --features "size_64 archive_le std bytecheck" >> results.txt
cargo test --no-default-features --features "size_16 archive_be std bytecheck" >> results.txt
cargo test --no-default-features --features "size_32 archive_be std bytecheck" >> results.txt
cargo test --no-default-features --features "size_64 archive_be std bytecheck" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy alloc" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy alloc bytecheck" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy std" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy std bytecheck" >> results.txt
cargo +nightly test --no-default-features --features "size_32 copy std bytecheck strict" >> results.txt
cargo test --release --no-default-features --features "size_32" >> results.txt
cargo test --release --no-default-features --features "size_32 alloc" >> results.txt
cargo test --release --no-default-features --features "size_32 bytecheck" >> results.txt
cargo test --release --no-default-features --features "size_32 std" >> results.txt
cargo test --release --no-default-features --features "size_32 std bytecheck" >> results.txt
cargo test --release --no-default-features --features "size_32 std bytecheck strict" >> results.txt
cargo test --release --no-default-features --features "size_16 archive_le std bytecheck" >> results.txt
cargo test --release --no-default-features --features "size_32 archive_le std bytecheck" >> results.txt
cargo test --release --no-default-features --features "size_64 archive_le std bytecheck" >> results.txt
cargo test --release --no-default-features --features "size_16 archive_be std bytecheck" >> results.txt
cargo test --release --no-default-features --features "size_32 archive_be std bytecheck" >> results.txt
cargo test --release --no-default-features --features "size_64 archive_be std bytecheck" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy alloc" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy alloc bytecheck" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy std" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy std bytecheck" >> results.txt
cargo +nightly test --release --no-default-features --features "size_32 copy std bytecheck strict" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 alloc" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 std" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 std bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 std bytecheck strict" >> results.txt
cargo +nightly miri test --no-default-features --features "size_16 archive_le std bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 archive_le std bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_64 archive_le std bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_16 archive_be std bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 archive_be std bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_64 archive_be std bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy alloc" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy alloc bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy std" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy std bytecheck" >> results.txt
cargo +nightly miri test --no-default-features --features "size_32 copy std bytecheck strict" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 alloc" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 bytecheck" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 std" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 std bytecheck" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 std bytecheck strict" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_16 archive_le std bytecheck" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 archive_le std bytecheck" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_16 archive_be std bytecheck" >> results.txt
wasm-pack test --node -- --no-default-features --features "wasm size_32 archive_be std bytecheck" >> results.txt
```
