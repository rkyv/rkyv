# TODO

- [x] Write archive implementations for HashMap and HashSet
- [x] Write derive macro for Archive
- [x] Add derive macro attribute to use the unit resolver (`#[derive(ArchiveCopy)]`)
- [ ] Add derive macro attributes for common traits that involve the archived and unarchived types (`#[archive(derive(Eq, Hash, PartialEq))]`) 
- [ ] Start thinking about archiving trait objects (`dyn Trait`)
- [ ] Documentation