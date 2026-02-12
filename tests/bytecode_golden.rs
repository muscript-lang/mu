use std::fs;

use muc::bytecode::{self, DecodeErrorCode, DecodedBytecode};
use muc::parser::parse_str;

fn decode_fixture(name: &str) -> Result<DecodedBytecode, muc::bytecode::DecodeError> {
    let bytes = fs::read(format!("tests/bytecode/{name}")).expect("fixture should exist");
    bytecode::decode(&bytes)
}

#[test]
fn minimal_fixture_decodes() {
    let decoded = decode_fixture("minimal_valid.mub").expect("minimal fixture should decode");
    assert_eq!(decoded.strings.len(), 0);
    assert_eq!(decoded.functions.len(), 1);
    assert_eq!(decoded.functions[0].arity, 0);
    assert_eq!(decoded.functions[0].captures, 0);
    assert_eq!(decoded.functions[0].code, vec![11]);
    assert_eq!(decoded.entry_fn, 0);
}

#[test]
fn encode_decode_roundtrip_is_deterministic() {
    let program = parse_str("@bc.r{F main:()->i32=v(f=l(x:i32):i32=c(+,x,1),c(f,4));}")
        .expect("program should parse");
    let encoded = bytecode::compile(&program).expect("program should compile");
    let decoded = bytecode::decode(&encoded).expect("encoded bytecode should decode");
    let reencoded = bytecode::encode(&decoded);
    assert_eq!(encoded, reencoded, "encode/decode should be deterministic");
}

#[test]
fn decode_rejects_corrupt_vectors_with_stable_codes() {
    let cases = [
        ("bad_header.mub", DecodeErrorCode::InvalidHeader),
        ("truncated.mub", DecodeErrorCode::Truncated),
        ("unknown_opcode.mub", DecodeErrorCode::UnknownOpcode),
        ("bad_jump_target.mub", DecodeErrorCode::InvalidJumpTarget),
    ];

    for (name, expected_code) in cases {
        let err = decode_fixture(name).expect_err("corrupt fixture should fail");
        assert_eq!(
            err.code, expected_code,
            "unexpected decode code for {name}: {}",
            err
        );
    }
}
