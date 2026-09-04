#[rustversion::attr(not(nightly), ignore)]
#[test]
#[cfg(not(miri))]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/archive_pointee_requires_unsafe.rs");
    t.pass("tests/ui/derive_visibility.rs");
    t.compile_fail("tests/ui/enum_as.rs");
    t.pass("tests/ui/raw_identifiers.rs");
    t.compile_fail("tests/ui/the_most_unhelpful_error.rs");
}
