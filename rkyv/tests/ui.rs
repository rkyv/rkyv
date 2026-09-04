#[rustversion::attr(not(nightly), ignore)]
#[test]
#[cfg(not(miri))]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/derive_visibility.rs");
    t.compile_fail("tests/ui/enum_as.rs");
    t.compile_fail("tests/ui/enum_discriminant_expression.rs");
    t.pass("tests/ui/enum_discriminant_attribute_shadow.rs");
    t.pass("tests/ui/enum_discriminant_bytecheck_shadow.rs");
    t.pass("tests/ui/enum_discriminant_core_shadow.rs");
    t.pass("tests/ui/enum_discriminant_derive_macro_shadow.rs");
    t.compile_fail("tests/ui/enum_discriminant_forwarded_attribute.rs");
    t.compile_fail("tests/ui/enum_discriminant_literal_range.rs");
    t.compile_fail("tests/ui/enum_discriminant_literal_suffix.rs");
    t.pass("tests/ui/enum_discriminant_literals.rs");
    t.compile_fail("tests/ui/enum_discriminant_macro.rs");
    t.compile_fail("tests/ui/enum_discriminant_negative.rs");
    t.pass("tests/ui/enum_discriminant_renamed_rkyv.rs");
    t.compile_fail("tests/ui/enum_discriminant_ufcs.rs");
    t.pass("tests/ui/raw_identifiers.rs");
    t.compile_fail("tests/ui/the_most_unhelpful_error.rs");
}
