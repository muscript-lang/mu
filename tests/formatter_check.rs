use std::fs;

use muc::fmt::parse_and_format;

#[test]
fn noncanonical_fixture_differs_from_canonical() {
    let input = fs::read_to_string("tests/fixtures/formatter/input_noncanonical.txt")
        .expect("input fixture readable");
    let expected = fs::read_to_string("tests/fixtures/formatter/input_noncanonical.expected.mu")
        .expect("expected fixture readable");

    let actual = parse_and_format(&input).expect("input should parse");

    assert_ne!(input, expected, "fixture must be non-canonical");
    assert_eq!(actual, expected, "formatter output mismatch");
}

#[test]
fn formatter_canonicalizes_effect_ordering() {
    let input = "@m.fx{F main:()->i32!{fs,io,fs}=0;}";
    let actual = parse_and_format(input).expect("input should parse");
    assert_eq!(actual, "@m.fx{F main:()->i32!{io,fs}=0;}\n");
}
