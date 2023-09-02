#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/test_*_invalid*.rs");
    t.pass("tests/ui/test_*_valid*.rs");
}
