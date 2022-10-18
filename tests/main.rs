mod expand;

#[test]
pub fn pass() {
    macrotest::expand("tests/expand/*.rs");
}
