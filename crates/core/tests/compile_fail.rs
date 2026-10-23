//! Typestate guarantees: an unfiltered UPDATE or DELETE must not become a `Query`.

#[test]
fn unfiltered_writes_do_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/unfiltered_*.rs");
}

/// Executor shortcuts: no `execute` on SELECT, no fetch without RETURNING,
/// no shortcut on an unfiltered UPDATE or DELETE.
#[test]
fn invalid_shortcuts_do_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui_shortcuts/*.rs");
}
