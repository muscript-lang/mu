use std::fs;

use muc::fmt::format_program;
use muc::parser::parse_str;

#[test]
fn parse_format_roundtrip_is_stable_for_fixtures() {
    let fixtures = [
        "tests/fixtures/parser/fixture1.mu",
        "tests/fixtures/parser/fixture2.mu",
        "tests/fixtures/parser/fixture3.mu",
    ];

    for fixture in fixtures {
        let src = fs::read_to_string(fixture).expect("fixture readable");
        let program = parse_str(&src).expect("fixture parses");
        let formatted_once = format_program(&program);

        let parsed_again = parse_str(&formatted_once).expect("formatted output parses");
        let formatted_twice = format_program(&parsed_again);

        assert_eq!(
            formatted_once, formatted_twice,
            "formatter is not stable for {fixture}"
        );
    }
}
