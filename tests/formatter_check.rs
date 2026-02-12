use std::fs;

use muc::fmt::{FmtMode, format_program_mode, parse_and_format, parse_and_format_mode};
use muc::parser::parse_str;

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

#[test]
fn compressed_formatter_emits_symtab_and_short_effect_atoms() {
    let input = "@m.fx{F main:()->i32!{io}={c(println,\"x\");0};}";
    let actual = parse_and_format_mode(input, FmtMode::Compressed).expect("input should parse");
    assert!(
        actual.contains("$["),
        "compressed output must include symbol table"
    );
    assert!(
        actual.contains("!{I}"),
        "compressed output must use short effect atoms"
    );
}

#[test]
fn compressed_formatter_is_idempotent() {
    let input = "@m.idem{:io=core.io;F main:()->i32!{io}={c(println,\"x\");0};}";
    let once = parse_and_format_mode(input, FmtMode::Compressed).expect("input should parse");
    let twice = parse_and_format_mode(&once, FmtMode::Compressed).expect("formatted should parse");
    assert_eq!(once, twice);
}

#[test]
fn readable_compressed_roundtrip_stable() {
    let input = "@m.rt{:io=core.io;T Opt=No|Yes(i32);F main:()->i32!{io}=m(Yes(1)){Yes(x)=>{c(println,\"ok\");x};No=>0;};}";
    let compressed = parse_and_format_mode(input, FmtMode::Compressed).expect("parse");
    let compressed2 = parse_and_format_mode(&compressed, FmtMode::Compressed).expect("parse");
    assert_eq!(compressed, compressed2);

    let readable = parse_and_format_mode(&compressed, FmtMode::Readable).expect("parse");
    let readable2 = parse_and_format_mode(&readable, FmtMode::Readable).expect("parse");
    assert_eq!(readable, readable2);
}

#[test]
fn compressed_symtab_is_deterministic_frequency_ordered() {
    let src_a = "@m.a{F zed:()->i32=0;F alpha:()->i32=c(zed);F main:()->i32=c(alpha);}";
    let src_b = "@m.a{F main:()->i32=c(alpha);F alpha:()->i32=c(zed);F zed:()->i32=0;}";

    let p_a = parse_str(src_a).expect("parse a");
    let p_b = parse_str(src_b).expect("parse b");
    let out_a = format_program_mode(&p_a, FmtMode::Compressed);
    let out_b = format_program_mode(&p_b, FmtMode::Compressed);

    let symtab_a = out_a
        .split("$[")
        .nth(1)
        .and_then(|s| s.split("];").next())
        .expect("symtab a");
    let symtab_b = out_b
        .split("$[")
        .nth(1)
        .and_then(|s| s.split("];").next())
        .expect("symtab b");

    assert_eq!(symtab_a, symtab_b, "symtab must be deterministic");
    assert_eq!(symtab_a, "alpha");
}

#[test]
fn compressed_symtab_excludes_core_forms() {
    let src = "@m.core{F main:()->i32={v(x=1,i(c(==,x,1),x,0))};}";
    let out = parse_and_format_mode(src, FmtMode::Compressed).expect("source should parse");
    let symtab = out
        .split("$[")
        .nth(1)
        .and_then(|s| s.split("];").next())
        .expect("symtab");
    for core in ["v", "i", "m", "l", "c", "a", "F", "T", "V", "E", "t", "f"] {
        assert!(
            !symtab.split(',').any(|entry| entry == core),
            "core form `{core}` should not be symtab-indexed: {symtab}"
        );
    }
}
