//! Typestate guarantees: an unfiltered UPDATE or DELETE must not become a `Query`.

#[test]
fn unfiltered_writes_do_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/unfiltered_*.rs");
}
