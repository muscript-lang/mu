use std::fs;

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

fn symtab_size_from_compressed(src: &str) -> usize {
    let Some(start) = src.find("$[") else {
        return 0;
    };
    let rest = &src[(start + 2)..];
    let Some(end) = rest.find(']') else {
        return 0;
    };
    let body = &rest[..end];
    if body.trim().is_empty() {
        0
    } else {
        body.split(',').count()
    }
}

#[test]
fn mu_dungeon_compressed_token_economy_targets() {
    let mut total_readable_bytes = 0usize;
    let mut total_compressed_bytes = 0usize;
    let mut total_readable_tokens = 0usize;
    let mut total_compressed_tokens = 0usize;
    let mut total_symtab_size = 0usize;
    let mut symref_widths = Vec::new();

    for path in [
        "apps/mu_dungeon/src/main.mu",
        "apps/mu_dungeon/src/rules.mu",
        "apps/mu_dungeon/src/token_pad.mu",
    ] {
        let src = fs::read_to_string(path).expect("mu dungeon source should exist");
        let program = parse_str(&src).expect("mu dungeon source should parse");

        let readable = format_program_mode(&program, FmtMode::Readable);
        let compressed = format_program_mode(&program, FmtMode::Compressed);

        let readable_bytes = readable.len();
        let compressed_bytes = compressed.len();
        let readable_tokens = lexer_token_economy_count(&readable);
        let compressed_tokens = lexer_token_economy_count(&compressed);
        let symtab_size = symtab_size_from_compressed(&compressed);
        let file_symrefs = tokenize(&compressed)
            .expect("compressed should lex")
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::SymRef(idx) => Some(idx.to_string().len()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let file_max_symref_width = file_symrefs.iter().copied().max().unwrap_or(0);
        let file_avg_symref_width = if file_symrefs.is_empty() {
            0.0
        } else {
            file_symrefs.iter().sum::<usize>() as f64 / file_symrefs.len() as f64
        };

        total_readable_bytes += readable_bytes;
        total_compressed_bytes += compressed_bytes;
        total_readable_tokens += readable_tokens;
        total_compressed_tokens += compressed_tokens;
        total_symtab_size += symtab_size;
        symref_widths.extend(file_symrefs);

        println!(
            "mu_dungeon token economy [{path}]: readable_bytes={readable_bytes} compressed_bytes={compressed_bytes} readable_tokens={readable_tokens} compressed_tokens={compressed_tokens} symtab_size={symtab_size} avg_symref_width={file_avg_symref_width:.2} max_symref_width={file_max_symref_width}"
        );
    }

    let max_symref_width = symref_widths.iter().copied().max().unwrap_or(0);
    let avg_symref_width = if symref_widths.is_empty() {
        0.0
    } else {
        symref_widths.iter().sum::<usize>() as f64 / symref_widths.len() as f64
    };

    println!(
        "mu_dungeon token economy [total]: readable_bytes={total_readable_bytes} compressed_bytes={total_compressed_bytes} readable_tokens={total_readable_tokens} compressed_tokens={total_compressed_tokens} symtab_size={total_symtab_size} avg_symref_width={avg_symref_width:.2} max_symref_width={max_symref_width}"
    );

    assert!(
        total_compressed_bytes * 100 <= total_readable_bytes * 70,
        "compressed bytes should be <= 70% of readable bytes"
    );
    assert!(
        total_compressed_tokens <= total_readable_tokens,
        "compressed token count should be <= readable token count"
    );
}
