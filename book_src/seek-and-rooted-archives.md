# Seek and rooted archives

[`Seek`](https://docs.rs/rkyv/latest/trait.Seek.html) is an extension trait for `Write` that enables
a writer to move throughout an archive, and allows the creation of *rooted archives*.

Normally, archiving a value will return the position that the archive was archived at. This means
that in most situations the user will have to store the position of the root object alongside the
archive data in order to access the archive properly. One possible solution to this problem would be
to store the offset of the root in the first few bytes of the archive, but this would still require
going back to fix those bytes up after finishing archiving.

If we have the ability to seek backwards in the archive, we can use it to archive the root object
at the start of the archive and guarantee that it will be located at position `0`. This is
essentially what the functions `archive_root` and `archive_ref_root` do, archiving the rest of the
data in the normal manner then backtracking to resolve the root object at the start of the archive.