use std::fs;

use muc::bytecode::compile;
use muc::fmt::{FmtMode, format_program_mode};
use muc::lexer::{TokenKind, tokenize};
use muc::parser::parse_str;

fn lexer_token_economy_count(src: &str) -> usize {
    tokenize(src)
        .expect("source should lex")
        .iter()
        .map(|token| match token.kind {
            TokenKind::SymRef(_)
            | TokenKind::Ident(_)
            | TokenKind::Int(_)
            | TokenKind::String(_) => 1,
            TokenKind::Arrow
            | TokenKind::FatArrow
            | TokenKind::EqEq
            | TokenKind::NotEq
            | TokenKind::Le
            | TokenKind::Ge => 2,
            TokenKind::Eof => 0,
            _ => 1,
        })
        .sum()
}

#[test]
fn token_efficiency_benchmark_bytecode_bytes_per_token() {
    let mut measured = Vec::new();
    for example in ["examples/hello.mu", "examples/json.mu", "examples/http.mu"] {
        let src = fs::read_to_string(example).expect("example source should exist");
        let token_count = lexer_token_economy_count(&src);
        let program = parse_str(&src).expect("example should parse");
        let bc = compile(&program).expect("example should compile");
        let ratio = bc.len() as f64 / token_count as f64;
        measured.push((example, token_count, bc.len(), ratio));
    }

    for (example, token_count, byte_len, ratio) in measured {
        println!(
            "token_efficiency {example}: tokens={token_count} bytecode_bytes={byte_len} bytes_per_token={ratio:.4}"
        );
        assert!(
            ratio <= 24.0,
            "token efficiency regression for {example}: bytes/token={ratio:.4}"
        );
    }
}

#[test]
fn compressed_mode_reduces_token_count_for_repeated_names() {
    let src = "@m.repeat{:io=core.io;F very_long_helper_name:()->i32!{io}={c(println,\"x\");0};F main:()->i32!{io}={c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);c(very_long_helper_name);0};}";
    let program = parse_str(src).expect("source should parse");
    let readable = format_program_mode(&program, FmtMode::Readable);
    let compressed = format_program_mode(&program, FmtMode::Compressed);

    let readable_tokens = lexer_token_economy_count(&readable);
    let compressed_tokens = lexer_token_economy_count(&compressed);

    assert!(
        compressed_tokens < readable_tokens,
        "compressed mode should reduce token count on repeated-name sources"
    );
}
