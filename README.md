# TODO

- [x] Write archive implementations for HashMap and HashSet
- [x] Write derive macro for Archive
- [x] Add derive macro attribute to use the self resolver (`#[archive(self)]`)
- [x] Add derive macro attribute to pass derive attributes (`#[archive(derive(Eq, Hash, PartialEq))]`) 
- [x] Start thinking about archiving trait objects (`dyn Trait`)
- [x] Add nightly feature and fix likely TODO
- [x] Add option to fix trait and type identifiers for stable hashes
- [x] Test out generic trait objects and figure out if they work (if not, is it feasible to add support?)
- [x] Add TypeName impls for basic types
- [x] Add TypeName derive
- [x] Do another pass and clean up traits (HashTypeName should hash the type name as literally as possible; think "Test<" + inner + ">")
- [x] Write macros for trait and impl generation
- [x] Add macro attributes to name derived archive types and ArchiveDyn traits
- [ ] Documentation
- [ ] Add crate descriptions and other information