# Release checklist

- [ ] Run `cargo clippy` and `cargo fmt` on all crates
- [ ] Generate documentation with `cargo doc --open` and make sure:
  - [ ] Every public item is documented
  - [ ] Every link is correct
- [ ] Make sure that `crates-io.md` and `README.md` are up to date with the most recent examples
- [ ] Run all tests, then run all tests in the test crate with all combinations of features
- [ ] Run all tests in release
- [ ] Bump version numbers and check that all crates have their dependencies updated to match
- [ ] Commit with the name `"Release X.X.X"` and push
- [ ] Merge development branch into `master`
- [ ] Publish crates
- [ ] Create release and tag version `"vX.X.X"` with a description that directly links to the issues related to the release
- [ ] Close milestone
- [ ] If sufficiently major, post on social media:
  - [ ] `/r/rust_gamedev`
  - [ ] `/r/rust`
  - [ ] Twitter