use muc::lexer::{LexErrorCode, TokenKind, tokenize};

#[test]
fn lexes_comments_and_escapes() {
    let src = "@m{/*block*/V s:s=\"a\\n\\\"b\\\\\";//line\n}";
    let tokens = tokenize(src).expect("source should lex");
    assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::At)));
    assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Colon)));
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Semicolon))
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::String(v) if v == "a\n\"b\\"))
    );
}

#[test]
fn rejects_leading_zero_int() {
    let err = tokenize("@m{V x:i32=01;}").expect_err("leading zero should fail");
    assert_eq!(err.code, LexErrorCode::InvalidIntLeadingZero);
    assert_eq!(err.code.as_str(), "E1006");
}

#[test]
fn rejects_unterminated_block_comment() {
    let err = tokenize("@m{/*").expect_err("unterminated block comment should fail");
    assert_eq!(err.code, LexErrorCode::UnterminatedBlockComment);
    assert_eq!(err.code.as_str(), "E1005");
}

#[test]
fn lexes_symtab_symref_brackets_and_sexpr() {
    let src = "@m{$[a,b];F #0:()->i32=[i t (#1 1) 0];}";
    let tokens = tokenize(src).expect("source should lex");
    assert!(
        tokens.iter().any(|t| matches!(t.kind, TokenKind::Dollar)),
        "should lex `$`"
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::SymRef(0))),
        "should lex symbol references"
    );
}
