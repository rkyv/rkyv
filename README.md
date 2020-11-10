# TODO

- [x] Write archive implementations for HashMap and HashSet
- [x] Write derive macro for Archive
- [x] Add derive macro attribute to use the self resolver (`#[archive(self)]`)
- [x] Add derive macro attribute to pass derive attributes (`#[archive(derive(Eq, Hash, PartialEq))]`) 
- [x] Start thinking about archiving trait objects (`dyn Trait`)
- [x] Add nightly feature and fix likely TODO
- [x] Add option to fix trait and type identifiers for stable hashes
- [ ] Test out generic trait objects and figure out if they work (if not, is it feasible to add support?)
- [ ] Write macros for trait and impl generation
- [ ] Documentation