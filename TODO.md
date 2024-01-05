- [ ] Update all dependencies to the latest versions
- [ ] Reformat line width
- [ ] Make core traits unsafe
- [ ] Get rid of out_field! if possible
- [ ] Re-enable RawArchivedVec. Move core data structure into shared struct and
      have RawArchivedVec and ArchivedVec be wrappers on top of it with
      different validation implementations.
- [ ] Rewrite subtree bounds checking to better encapsulate the actual pointer checking (pass a closure to ArchiveContextExt)
- [ ] Try to get rid of fully-qualified function call syntax for `serialize_value`, `pos`, and `deserialize`
- [ ] Add a `-Zrandomize-layout` run to the test suite. Some rkyv types are missing `#[repr(c)]` even with `strict` turned on.
