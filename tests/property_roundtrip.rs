use std::fs;

use muc::bytecode;
use muc::fmt::{FmtMode, format_program, format_program_mode};
use muc::parser::parse_str;

#[test]
fn formatter_is_idempotent_on_examples() {
    for path in ["examples/hello.mu", "examples/json.mu", "examples/http.mu"] {
        let src = fs::read_to_string(path).expect("example should exist");
        let p1 = parse_str(&src).expect("example should parse");
        let f1 = format_program(&p1);
        let p2 = parse_str(&f1).expect("formatted example should parse");
        let f2 = format_program(&p2);
        assert_eq!(f1, f2, "formatter must be idempotent for {path}");
    }
}

#[test]
fn bytecode_encode_decode_is_idempotent_on_examples() {
    for path in ["examples/hello.mu", "examples/json.mu", "examples/http.mu"] {
        let src = fs::read_to_string(path).expect("example should exist");
        let program = parse_str(&src).expect("example should parse");
        let encoded = bytecode::compile(&program).expect("example should compile");
        let decoded = bytecode::decode(&encoded).expect("encoded should decode");
        let reencoded = bytecode::encode(&decoded);
        assert_eq!(encoded, reencoded, "bytecode roundtrip changed for {path}");
    }
}

#[test]
fn formatter_modes_are_each_idempotent_on_examples() {
    for path in ["examples/hello.mu", "examples/json.mu", "examples/http.mu"] {
        let src = fs::read_to_string(path).expect("example should exist");
        let p1 = parse_str(&src).expect("example should parse");

        let readable1 = format_program_mode(&p1, FmtMode::Readable);
        let readable2 = format_program_mode(
            &parse_str(&readable1).expect("readable should parse"),
            FmtMode::Readable,
        );
        assert_eq!(
            readable1, readable2,
            "readable idempotence failed for {path}"
        );

        let compressed1 = format_program_mode(&p1, FmtMode::Compressed);
        let compressed2 = format_program_mode(
            &parse_str(&compressed1).expect("compressed should parse"),
            FmtMode::Compressed,
        );
        assert_eq!(
            compressed1, compressed2,
            "compressed idempotence failed for {path}"
        );
    }
}
