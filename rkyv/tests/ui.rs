#[rustversion::attr(not(nightly), ignore)]
#[test]
#[cfg(not(miri))]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/derive_visibility.rs");
    t.pass("tests/ui/deserialize_unsized_unsafe_impl.rs");
    t.compile_fail("tests/ui/deserialize_unsized_requires_unsafe_impl.rs");
    t.compile_fail("tests/ui/enum_as.rs");
    t.pass("tests/ui/raw_identifiers.rs");
    t.compile_fail("tests/ui/the_most_unhelpful_error.rs");
}
