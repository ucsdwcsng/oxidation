#![cfg(feature = "derive")]

#[test]
fn extension_derive_emits_consumer_diagnostics() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/derive/*.rs");
}
