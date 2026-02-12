use std::fs;

use muc::fmt::{FmtMode, format_program_mode};
use muc::parser::parse_str;

fn tokenish_count(src: &str) -> usize {
    src.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .count()
}

#[test]
fn signal_reactor_compressed_is_at_most_95_percent_of_readable() {
    let src = fs::read_to_string("apps/signal_reactor/src/signal_reactor.mu")
        .expect("signal reactor source should exist");
    let program = parse_str(&src).expect("signal reactor source should parse");

    let readable = format_program_mode(&program, FmtMode::Readable);
    let compressed = format_program_mode(&program, FmtMode::Compressed);

    let readable_bytes = readable.len();
    let compressed_bytes = compressed.len();
    let readable_tokens = tokenish_count(&readable);
    let compressed_tokens = tokenish_count(&compressed);

    println!(
        "signal_reactor token economy: readable_bytes={readable_bytes} compressed_bytes={compressed_bytes} readable_tokens={readable_tokens} compressed_tokens={compressed_tokens}"
    );

    assert!(
        compressed_bytes * 100 <= readable_bytes * 95,
        "compressed bytes should be <= 95% of readable bytes"
    );
}
