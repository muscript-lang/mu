use std::fs;

use muc::bytecode::compile;
use muc::lexer::{TokenKind, tokenize};
use muc::parser::parse_str;

#[test]
fn token_efficiency_benchmark_bytecode_bytes_per_token() {
    let mut measured = Vec::new();
    for example in ["examples/hello.mu", "examples/json.mu", "examples/http.mu"] {
        let src = fs::read_to_string(example).expect("example source should exist");
        let tokens = tokenize(&src).expect("example should lex");
        let token_count = tokens
            .iter()
            .filter(|t| !matches!(t.kind, TokenKind::Eof))
            .count();
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
